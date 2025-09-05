use crate::state::{AppState, TaskUpdate};
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
    
    // Send current tasks on connection
    let current_tasks = state.get_all_tasks().await;
    for task in current_tasks {
        let update = TaskUpdate {
            task_id: task.id,
            status: match task.status.as_str() {
                "pending" => crate::state::TaskStatus::Pending,
                "converting" => crate::state::TaskStatus::Converting,
                "completed" => crate::state::TaskStatus::Completed,
                "cancelled" => crate::state::TaskStatus::Cancelled,
                s if s.starts_with("failed") => crate::state::TaskStatus::Failed(s.to_string()),
                _ => crate::state::TaskStatus::Pending,
            },
            progress: task.progress,
            speed: "0x".to_string(),
            eta: "Unknown".to_string(),
        };
        
        if let Ok(message) = serde_json::to_string(&update) {
            if sender.send(Message::Text(message)).await.is_err() {
                warn!("Failed to send initial task update");
                return;
            }
        }
    }
    
    // Handle incoming messages and broadcast updates
    loop {
        tokio::select! {
            // Handle incoming WebSocket messages
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Handle client messages (e.g., ping, subscribe to specific tasks)
                        if text == "ping"
                            && sender.send(Message::Text("pong".to_string())).await.is_err() {
                                break;
                            }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket connection closed by client");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
            
            // Handle task updates
            update = task_updates.recv() => {
                match update {
                    Ok(task_update) => {
                        if let Ok(message) = serde_json::to_string(&task_update) {
                            if sender.send(Message::Text(message)).await.is_err() {
                                warn!("Failed to send task update, client disconnected");
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket client lagged behind by {} messages", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        }
    }
    
    info!("WebSocket connection closed");
}
