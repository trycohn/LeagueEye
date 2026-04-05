use reqwest::Client;
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
        Self {
            client: Client::new(),
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

    // --- Live game enrichment ---

    pub async fn enrich_live_game(&self, request: &EnrichLiveRequest) -> Result<LiveGameData, String> {
        self.post("/api/live/enrich", request).await
    }
}

/// Request sent from client to server for live game enrichment
#[derive(serde::Serialize)]
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

#[derive(serde::Serialize)]
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
