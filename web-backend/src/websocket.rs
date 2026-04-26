use crate::state::{AppState, TaskStatus};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| websocket_connection(socket, state))
}

async fn websocket_connection(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut task_updates = state.subscribe_to_updates();

    info!("New WebSocket connection established");

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if text == "ping"
                            && sender.send(Message::Text("pong".to_string().into())).await.is_err() {
                                break;
                            }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket connection closed by client");
                        break;
                    }
                    Some(Err(error)) => {
                        let error_message = error.to_string();
                        if error_message.contains("Connection reset") || error_message.contains("close handshake") {
                            info!("WebSocket client disconnected: {}", error);
                        } else {
                            error!("WebSocket error: {}", error);
                        }
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
            update = task_updates.recv() => {
                match update {
                    Ok(task_update) => {
                        let message_type = match &task_update.status {
                            TaskStatus::Completed => "task_completed",
                            TaskStatus::Failed(_) => "task_failed",
                            _ => "progress_update",
                        };

                        let payload = serde_json::json!({
                            "type": message_type,
                            "task_id": task_update.task_id,
                            "status": task_update.status,
                            "progress": task_update.progress,
                            "speed": task_update.speed,
                            "eta": task_update.eta,
                        });

                        if sender
                            .send(Message::Text(payload.to_string().into()))
                            .await
                            .is_err()
                        {
                            warn!("Failed to send task update, client disconnected");
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        warn!("WebSocket client lagged by {} messages", count);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    info!("WebSocket connection closed");
}
