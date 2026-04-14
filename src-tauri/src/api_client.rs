use reqwest::Client;
use tauri::{AppHandle, Emitter};
use leagueeye_shared::models::*;

/// HTTP client that talks to the LeagueEye server.
/// Replaces direct Riot API calls in the client.
pub struct ServerApiClient {
    client: Client,
    base_url: String,
}

impl Clone for ServerApiClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
        }
    }
}

impl ServerApiClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .danger_accept_invalid_certs(base_url.starts_with("https://"))
            .build()
            .unwrap_or_default();
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Ошибка соединения с сервером: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            // Try to extract the error string (server returns plain strings)
            let error_msg = if body.starts_with('"') {
                serde_json::from_str::<String>(&body).unwrap_or(body)
            } else {
                body
            };
            return Err(error_msg);
        }

        response.json::<T>().await
            .map_err(|e| format!("Ошибка парсинга ответа: {}", e))
    }

    async fn post<T: serde::de::DeserializeOwned, B: serde::Serialize>(&self, path: &str, body: &B) -> Result<T, String> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("Ошибка соединения с сервером: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let error_msg = if body.starts_with('"') {
                serde_json::from_str::<String>(&body).unwrap_or(body)
            } else {
                body
            };
            return Err(error_msg);
        }

        response.json::<T>().await
            .map_err(|e| format!("Ошибка парсинга ответа: {}", e))
    }

    // --- Player endpoints ---

    pub async fn search_player(&self, game_name: &str, tag_line: &str) -> Result<PlayerProfile, String> {
        self.get(&format!("/api/players/{}/{}", game_name, tag_line)).await
    }

    pub async fn get_mastery(&self, puuid: &str) -> Result<Vec<MasteryInfo>, String> {
        self.get(&format!("/api/players/{}/mastery", puuid)).await
    }

    pub async fn get_matches_and_stats(&self, puuid: &str) -> Result<MatchesAndStats, String> {
        self.get(&format!("/api/players/{}/matches", puuid)).await
    }

    pub async fn get_matchups(&self, puuid: &str) -> Result<Vec<MatchupStat>, String> {
        self.get(&format!("/api/players/{}/matchups", puuid)).await
    }

    pub async fn get_frequent_teammates(&self, puuid: &str) -> Result<Vec<FrequentTeammate>, String> {
        self.get(&format!("/api/players/{}/frequent-teammates", puuid)).await
    }

    pub async fn load_more_matches(&self, puuid: &str, offset: usize, limit: usize) -> Result<Vec<MatchSummary>, String> {
        let result: MatchesAndStats = self.get(&format!(
            "/api/players/{}/matches?offset={}&limit={}", puuid, offset, limit
        )).await?;
        Ok(result.matches)
    }

    // --- Match endpoints ---

    pub async fn get_match_detail(&self, match_id: &str) -> Result<MatchDetail, String> {
        self.get(&format!("/api/matches/{}", match_id)).await
    }

    // --- Global endpoints ---

    pub async fn get_global_dashboard(&self) -> Result<GlobalDashboardData, String> {
        self.get("/api/global/dashboard").await
    }

    // --- Live game enrichment ---

    pub async fn enrich_live_game(&self, request: &EnrichLiveRequest) -> Result<LiveGameData, String> {
        // 5s timeout: enrich runs in background during champ select, so we
        // don't want it to hang indefinitely if the server is overloaded.
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.post::<LiveGameData, _>("/api/live/enrich", request),
        )
        .await
        .map_err(|_| "Enrich запрос превысил таймаут (5с)".to_string())?
    }

    // --- AI Coach streaming ---

    pub async fn stream_coaching(
        &self,
        app: &AppHandle,
        ctx: &CoachingContext,
    ) -> Result<(), String> {
        let url = format!("{}/api/coach/stream", self.base_url);

        let response = self.client
            .post(&url)
            .json(ctx)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Ошибка соединения с сервером: {}", e);
                let _ = app.emit("coach-stream", CoachStreamPayload {
                    kind: "error".to_string(),
                    text: Some(msg.clone()),
                });
                msg
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let msg = format!("Ошибка сервера ({}): {}", status, body);
            let _ = app.emit("coach-stream", CoachStreamPayload {
                kind: "error".to_string(),
                text: Some(msg.clone()),
            });
            return Err(msg);
        }

        // Read SSE stream from server line-by-line and forward as Tauri events.
        // Using single-newline parsing to avoid buffering delays from waiting for "\n\n".
        let mut buffer = String::new();
        let mut response = response;

        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim_end().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() || line.starts_with(':') {
                    continue;
                }
                if let Some(data) = line.strip_prefix("data:") {
                    let data = data.trim_start();
                    if let Ok(payload) = serde_json::from_str::<CoachStreamPayload>(data) {
                        let _ = app.emit("coach-stream", &payload);
                    }
                }
            }
        }

        Ok(())
    }

    // --- Draft Helper streaming (reuses coach endpoint with draft-* event kinds) ---

    pub async fn stream_draft_coaching(
        &self,
        app: &AppHandle,
        ctx: &CoachingContext,
    ) -> Result<(), String> {
        let url = format!("{}/api/coach/stream", self.base_url);

        let response = self.client
            .post(&url)
            .json(ctx)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Ошибка соединения с сервером: {}", e);
                let _ = app.emit("coach-stream", CoachStreamPayload {
                    kind: "draft-error".to_string(),
                    text: Some(msg.clone()),
                });
                msg
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let msg = format!("Ошибка сервера ({}): {}", status, body);
            let _ = app.emit("coach-stream", CoachStreamPayload {
                kind: "draft-error".to_string(),
                text: Some(msg.clone()),
            });
            return Err(msg);
        }

        let mut buffer = String::new();
        let mut response = response;

        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim_end().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() || line.starts_with(':') {
                    continue;
                }
                if let Some(data) = line.strip_prefix("data:") {
                    let data = data.trim_start();
                    if let Ok(payload) = serde_json::from_str::<CoachStreamPayload>(data) {
                        // Remap kind: start→draft-start, delta→draft-delta, end→draft-end, error→draft-error
                        let draft_kind = match payload.kind.as_str() {
                            "start" => "draft-start",
                            "delta" => "draft-delta",
                            "end" => "draft-end",
                            "error" => "draft-error",
                            other => other,
                        };
                        let _ = app.emit("coach-stream", CoachStreamPayload {
                            kind: draft_kind.to_string(),
                            text: payload.text,
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

/// Request sent from client to server for live game enrichment
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EnrichLiveRequest {
    pub phase: String,
    pub players: Vec<EnrichLivePlayer>,
    pub bans: Vec<LiveBan>,
    pub game_time: Option<i64>,
    pub timer: Option<LiveTimer>,
    pub my_puuid: Option<String>,
    pub queue_id: Option<i32>,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EnrichLivePlayer {
    pub puuid: Option<String>,
    pub game_name: Option<String>,
    pub tag_line: Option<String>,
    pub champion_id: i64,
    pub assigned_position: Option<String>,
    pub spell1_id: i32,
    pub spell2_id: i32,
    pub team_id: i32,
    pub is_picking: bool,
}
