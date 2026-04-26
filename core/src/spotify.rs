use crate::AppConfig;
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER, USER_AGENT};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

const SPOTIFY_WEB_TOKEN_URL: &str =
    "https://open.spotify.com/get_access_token?reason=transport&productType=web_player";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyPlaylist {
    pub id: String,
    pub title: String,
    pub owner: String,
    pub total_tracks: usize,
    pub tracks: Vec<SpotifyTrack>,
    pub complete: bool,
    pub notice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyTrack {
    pub id: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub spotify_url: String,
}

impl SpotifyTrack {
    pub fn display_title(&self) -> String {
        let artists = self.artists.join(", ");
        if artists.is_empty() {
            self.title.clone()
        } else {
            format!("{artists} - {}", self.title)
        }
    }

    pub fn search_query(&self, config: &AppConfig) -> String {
        let mut pieces = Vec::new();
        if !self.artists.is_empty() {
            pieces.push(self.artists.join(" "));
        }
        pieces.push(self.title.clone());
        if !config.spotify.search_suffix.trim().is_empty() {
            pieces.push(config.spotify.search_suffix.trim().to_string());
        }

        let query = pieces.join(" ");
        format!("ytsearch1:{}", collapse_whitespace(&query))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpotifyWebTokenResponse {
    access_token: String,
}

pub async fn resolve_playlist(config: &AppConfig, url: &str) -> Result<SpotifyPlaylist> {
    if !config.spotify.enabled {
        return Err(anyhow!(
            "Spotify playlist support is disabled in config.spotify.enabled"
        ));
    }

    let playlist_id = extract_spotify_playlist_id(url)
        .ok_or_else(|| anyhow!("Invalid Spotify playlist URL: {url}"))?;
    let resolver = SpotifyResolver::new(config.clone())?;

    let mut last_error = None;
    if let Ok(token) = resolver.fetch_web_player_token().await {
        match resolver
            .resolve_with_public_api(&playlist_id, &token.access_token)
            .await
        {
            Ok(playlist) => return Ok(playlist),
            Err(error) => last_error = Some(error),
        }
    }

    if !config.spotify.fallback_to_page_scrape {
        return Err(last_error.unwrap_or_else(|| {
            anyhow!("Spotify playlist resolution failed and HTML fallback is disabled")
        }));
    }

    let mut playlist = resolver.resolve_from_page(url, &playlist_id).await?;
    if playlist.total_tracks > playlist.tracks.len() {
        playlist.notice = Some(format!(
            "Resolved {} of {} tracks from the public page fallback. Configure Spotify API access for complete pagination.",
            playlist.tracks.len(),
            playlist.total_tracks
        ));
    } else if let Some(error) = last_error {
        playlist.notice = Some(format!(
            "Used the public Spotify page fallback after the API path failed: {error}"
        ));
    }

    Ok(playlist)
}

pub fn is_valid_spotify_playlist_url(url: &str) -> bool {
    extract_spotify_playlist_id(url).is_some()
}

pub fn extract_spotify_playlist_id(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if let Some(value) = trimmed.strip_prefix("spotify:playlist:") {
        return (!value.is_empty()).then_some(value.to_string());
    }

    let without_query = trimmed.split('?').next().unwrap_or(trimmed);
    let segments = without_query.split('/').collect::<Vec<_>>();
    for window in segments.windows(2) {
        if window[0] == "playlist" && !window[1].is_empty() {
            return Some(window[1].to_string());
        }
    }

    None
}

struct SpotifyResolver {
    client: Client,
    config: AppConfig,
}

impl SpotifyResolver {
    fn new(config: AppConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Rustify/0.1 (+https://open.spotify.com)"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(45))
            .build()
            .context("Failed to create Spotify HTTP client")?;

        Ok(Self { client, config })
    }

    async fn fetch_web_player_token(&self) -> Result<SpotifyWebTokenResponse> {
        self.get_json(
            self.client.get(SPOTIFY_WEB_TOKEN_URL),
            "spotify web player token",
        )
        .await
    }

    async fn resolve_with_public_api(&self, playlist_id: &str, token: &str) -> Result<SpotifyPlaylist> {
        let meta_url = format!("https://api.spotify.com/v1/playlists/{playlist_id}");
        let metadata: Value = self
            .get_json(
                self.authorized_get(&meta_url, token).query(&[
                    (
                        "fields",
                        "id,name,description,owner(display_name,id),tracks(total)".to_string(),
                    ),
                    ("market", self.market().unwrap_or_default()),
                ]),
                "spotify playlist metadata",
            )
            .await?;

        let title = metadata["name"]
            .as_str()
            .unwrap_or("Spotify Playlist")
            .to_string();
        let owner = metadata["owner"]["display_name"]
            .as_str()
            .or_else(|| metadata["owner"]["id"].as_str())
            .unwrap_or("Spotify")
            .to_string();
        let total_tracks = metadata["tracks"]["total"].as_u64().unwrap_or_default() as usize;

        let page_size = self.config.spotify.page_size.clamp(1, 100);
        let mut offset = 0usize;
        let mut tracks = Vec::with_capacity(total_tracks.min(page_size));

        while offset < total_tracks.max(1) {
            let page_url = format!("https://api.spotify.com/v1/playlists/{playlist_id}/tracks");
            let page: Value = self
                .get_json(
                    self.authorized_get(&page_url, token).query(&[
                        (
                            "fields",
                            "items(is_local,track(id,name,uri,duration_ms,artists(name),album(name),external_urls(spotify))),limit,next,total"
                                .to_string(),
                        ),
                        ("limit", page_size.to_string()),
                        ("offset", offset.to_string()),
                        ("market", self.market().unwrap_or_default()),
                    ]),
                    "spotify playlist page",
                )
                .await?;

            let items = page["items"].as_array().cloned().unwrap_or_default();
            if items.is_empty() {
                break;
            }

            for item in items {
                if item["is_local"].as_bool().unwrap_or(false) {
                    continue;
                }

                if let Some(track) = parse_api_track(&item["track"]) {
                    tracks.push(track);
                }
            }

            if page["next"].is_null() {
                break;
            }

            offset += page_size;
            self.pace_requests().await;
        }

        Ok(SpotifyPlaylist {
            id: playlist_id.to_string(),
            title,
            owner,
            total_tracks,
            complete: tracks.len() >= total_tracks && total_tracks > 0,
            tracks,
            notice: None,
        })
    }

    async fn resolve_from_page(&self, url: &str, playlist_id: &str) -> Result<SpotifyPlaylist> {
        let html = self
            .get_text(self.client.get(url), "spotify playlist page")
            .await?;
        let encoded_state = capture_script_payload(&html, "initialState")
            .ok_or_else(|| anyhow!("Spotify page did not expose initialState data"))?;
        let decoded = BASE64_STANDARD
            .decode(encoded_state.as_bytes())
            .context("Failed to decode Spotify initialState payload")?;
        let state: Value = serde_json::from_slice(&decoded)
            .context("Failed to parse Spotify initialState payload")?;

        let entity_key = format!("spotify:playlist:{playlist_id}");
        let playlist = &state["entities"]["items"][&entity_key];
        if playlist.is_null() {
            return Err(anyhow!("Spotify page did not contain playlist metadata"));
        }

        let items = playlist["content"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let tracks = items
            .iter()
            .filter_map(|item| parse_page_track(&item["itemV2"]["data"]))
            .collect::<Vec<_>>();
        let total_tracks = playlist["content"]["totalCount"]
            .as_u64()
            .unwrap_or(tracks.len() as u64) as usize;

        Ok(SpotifyPlaylist {
            id: playlist_id.to_string(),
            title: playlist["name"]
                .as_str()
                .unwrap_or("Spotify Playlist")
                .to_string(),
            owner: playlist["ownerV2"]["data"]["name"]
                .as_str()
                .unwrap_or("Spotify")
                .to_string(),
            total_tracks,
            complete: tracks.len() >= total_tracks && total_tracks > 0,
            tracks,
            notice: None,
        })
    }

    fn authorized_get(&self, url: &str, token: &str) -> reqwest::RequestBuilder {
        self.client.get(url).bearer_auth(token)
    }

    fn market(&self) -> Option<String> {
        self.config
            .spotify
            .market
            .as_ref()
            .filter(|value| value.as_str() != "from_token")
            .cloned()
    }

    async fn pace_requests(&self) {
        if self.config.rate_limits.request_delay_ms > 0 {
            sleep(Duration::from_millis(self.config.rate_limits.request_delay_ms)).await;
        }
    }

    async fn get_text(
        &self,
        request: reqwest::RequestBuilder,
        label: &str,
    ) -> Result<String> {
        let mut attempt = 0u32;
        loop {
            let response = request
                .try_clone()
                .ok_or_else(|| anyhow!("Failed to clone HTTP request for {label}"))?
                .send()
                .await
                .with_context(|| format!("Failed to send {label} request"))?;

            if response.status().is_success() {
                return response
                    .text()
                    .await
                    .with_context(|| format!("Failed to read {label} response body"));
            }

            if should_retry_status(response.status()) && attempt < self.config.rate_limits.max_retries
            {
                let delay = retry_delay(&self.config, attempt, response.headers());
                attempt += 1;
                sleep(delay).await;
                continue;
            }

            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "{label} request failed with status {}: {}",
                status,
                body.trim()
            ));
        }
    }

    async fn get_json<T>(&self, request: reqwest::RequestBuilder, label: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut attempt = 0u32;
        loop {
            let response = request
                .try_clone()
                .ok_or_else(|| anyhow!("Failed to clone HTTP request for {label}"))?
                .send()
                .await
                .with_context(|| format!("Failed to send {label} request"))?;

            if response.status().is_success() {
                return response
                    .json::<T>()
                    .await
                    .with_context(|| format!("Failed to decode {label} response"));
            }

            if should_retry_status(response.status()) && attempt < self.config.rate_limits.max_retries
            {
                let delay = retry_delay(&self.config, attempt, response.headers());
                attempt += 1;
                sleep(delay).await;
                continue;
            }

            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "{label} request failed with status {}: {}",
                status,
                body.trim()
            ));
        }
    }
}

fn parse_api_track(track: &Value) -> Option<SpotifyTrack> {
    if track.is_null() {
        return None;
    }

    let id = track["id"]
        .as_str()
        .map(str::to_string)
        .or_else(|| spotify_id_from_uri(track["uri"].as_str()?))?;

    Some(SpotifyTrack {
        id: id.clone(),
        title: track["name"].as_str()?.to_string(),
        artists: track["artists"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|artist| artist["name"].as_str().map(str::to_string))
            .collect(),
        album: track["album"]["name"].as_str().map(str::to_string),
        duration_ms: track["duration_ms"].as_u64(),
        spotify_url: track["external_urls"]["spotify"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("https://open.spotify.com/track/{id}")),
    })
}

fn parse_page_track(track: &Value) -> Option<SpotifyTrack> {
    if track.is_null() {
        return None;
    }

    let id = spotify_id_from_uri(track["uri"].as_str()?)?;

    Some(SpotifyTrack {
        id: id.clone(),
        title: track["name"].as_str()?.to_string(),
        artists: track["artists"]["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|artist| artist["profile"]["name"].as_str().map(str::to_string))
            .collect(),
        album: track["albumOfTrack"]["name"].as_str().map(str::to_string),
        duration_ms: track["duration"]["totalMilliseconds"].as_u64(),
        spotify_url: format!("https://open.spotify.com/track/{id}"),
    })
}

fn capture_script_payload<'a>(html: &'a str, id: &str) -> Option<&'a str> {
    let start_marker = format!(r#"<script id="{id}" type="text/plain">"#);
    let start = html.find(&start_marker)? + start_marker.len();
    let tail = &html[start..];
    let end = tail.find("</script>")?;
    Some(tail[..end].trim())
}

fn spotify_id_from_uri(uri: &str) -> Option<String> {
    uri.rsplit(':').next().map(str::to_string)
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn should_retry_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn retry_delay(config: &AppConfig, attempt: u32, headers: &HeaderMap) -> Duration {
    if let Some(value) = headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
    {
        return Duration::from_secs(value);
    }

    let multiplier = 2u64.saturating_pow(attempt.min(8));
    Duration::from_millis(config.rate_limits.backoff_base_ms.saturating_mul(multiplier))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_spotify_playlist_id_from_web_url() {
        assert_eq!(
            extract_spotify_playlist_id(
                "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M?si=abc"
            ),
            Some("37i9dQZF1DXcBWIGoYBM5M".to_string())
        );
    }

    #[test]
    fn parses_spotify_playlist_id_from_uri() {
        assert_eq!(
            extract_spotify_playlist_id("spotify:playlist:37i9dQZF1DXcBWIGoYBM5M"),
            Some("37i9dQZF1DXcBWIGoYBM5M".to_string())
        );
    }

    #[test]
    fn builds_youtube_search_query() {
        let track = SpotifyTrack {
            id: "1".to_string(),
            title: "Nights".to_string(),
            artists: vec!["Frank Ocean".to_string()],
            album: None,
            duration_ms: Some(300_000),
            spotify_url: "https://open.spotify.com/track/1".to_string(),
        };

        let query = track.search_query(&AppConfig::default());
        assert!(query.starts_with("ytsearch1:"));
        assert!(query.contains("Frank Ocean"));
        assert!(query.contains("Nights"));
    }
}
