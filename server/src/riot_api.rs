use reqwest::Client;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant, sleep};

use leagueeye_shared::models::*;

const PLATFORM_URL: &str = "https://ru.api.riotgames.com";
const REGIONAL_URL: &str = "https://europe.api.riotgames.com";

// Лимиты с небольшим запасом
const PER_SECOND_LIMIT: usize = 18;   // от 20
const PER_TWO_MIN_LIMIT: usize = 95;  // от 100

struct RateLimiter {
    timestamps: VecDeque<Instant>,
}

impl RateLimiter {
    fn new() -> Self {
        Self { timestamps: VecDeque::new() }
    }

    fn check_and_reserve(&mut self) -> Option<Duration> {
        let now = Instant::now();

        while self.timestamps.front().map(|t| now.duration_since(*t) > Duration::from_secs(120)).unwrap_or(false) {
            self.timestamps.pop_front();
        }

        let count_2min = self.timestamps.len();
        let count_1s = self.timestamps.iter()
            .filter(|t| now.duration_since(**t) < Duration::from_secs(1))
            .count();

        if count_1s >= PER_SECOND_LIMIT {
            let oldest_1s = self.timestamps.iter()
                .rev()
                .nth(count_1s - 1)
                .copied()
                .unwrap_or(now);
            let wait = Duration::from_secs(1).saturating_sub(now.duration_since(oldest_1s)) + Duration::from_millis(10);
            return Some(wait);
        }

        if count_2min >= PER_TWO_MIN_LIMIT {
            let oldest = self.timestamps.front().copied().unwrap_or(now);
            let wait = Duration::from_secs(120).saturating_sub(now.duration_since(oldest)) + Duration::from_millis(10);
            return Some(wait);
        }

        self.timestamps.push_back(Instant::now());
        None
    }
}

#[derive(Clone)]
pub struct RiotApiClient {
    client: Client,
    api_key: Arc<String>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

impl RiotApiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key: Arc::new(api_key),
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new())),
        }
    }

    async fn rate_limit(&self) {
        loop {
            let wait = self.rate_limiter.lock().await.check_and_reserve();
            match wait {
                None => break,
                Some(duration) => sleep(duration).await,
            }
        }
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, String> {
        self.rate_limit().await;
        self.get_raw(url).await
    }

    /// Perform a GET request WITHOUT going through the rate limiter.
    /// Used for live-game enrichment (ranks, puuid resolution) which is
    /// latency-sensitive and runs in background — we rely on Riot 429 retry
    /// instead of pre-emptive throttling to avoid blocking the draft UI.
    async fn get_raw<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, String> {

        let response = self
            .client
            .get(url)
            .header("X-Riot-Token", self.api_key.as_str())
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(2);

            sleep(Duration::from_secs(retry_after)).await;
            return Box::pin(self.get_raw(url)).await;
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err("Игрок не найден".to_string());
        }

        if status == reqwest::StatusCode::FORBIDDEN {
            return Err("API-ключ истёк или недействителен".to_string());
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status.as_u16(), body));
        }

        response
            .json::<T>()
            .await
            .map_err(|e| format!("Parse error: {}", e))
    }

    pub async fn get_account_by_riot_id(&self, game_name: &str, tag_line: &str) -> Result<RiotAccount, String> {
        let url = format!("{}/riot/account/v1/accounts/by-riot-id/{}/{}", REGIONAL_URL, game_name, tag_line);
        self.get(&url).await
    }

    pub async fn get_summoner_by_puuid(&self, puuid: &str) -> Result<Summoner, String> {
        let url = format!("{}/lol/summoner/v4/summoners/by-puuid/{}", PLATFORM_URL, puuid);
        self.get(&url).await
    }

    pub async fn get_league_entries(&self, summoner_id: &str) -> Result<Vec<LeagueEntry>, String> {
        let url = format!("{}/lol/league/v4/entries/by-summoner/{}", PLATFORM_URL, summoner_id);
        self.get(&url).await
    }

    pub async fn get_league_entries_by_puuid(&self, puuid: &str) -> Result<Vec<LeagueEntry>, String> {
        let url = format!("{}/lol/league/v4/entries/by-puuid/{}", PLATFORM_URL, puuid);
        self.get(&url).await
    }

    pub async fn get_champion_mastery_top(&self, puuid: &str, count: u32) -> Result<Vec<ChampionMastery>, String> {
        let url = format!("{}/lol/champion-mastery/v4/champion-masteries/by-puuid/{}/top?count={}", PLATFORM_URL, puuid, count);
        self.get(&url).await
    }

    pub async fn get_match_ids(&self, puuid: &str, start: u32, count: u32) -> Result<Vec<String>, String> {
        let url = format!("{}/lol/match/v5/matches/by-puuid/{}/ids?start={}&count={}", REGIONAL_URL, puuid, start, count);
        self.get(&url).await
    }

    pub async fn get_all_match_ids(&self, puuid: &str, max: usize) -> Result<Vec<String>, String> {
        let mut all_ids = Vec::new();
        let mut start = 0u32;
        loop {
            let batch = self.get_match_ids(puuid, start, 100).await?;
            if batch.is_empty() { break; }
            let len = batch.len();
            all_ids.extend(batch);
            if all_ids.len() >= max || len < 100 { break; }
            start += len as u32;
        }
        all_ids.truncate(max);
        Ok(all_ids)
    }

    pub async fn get_match(&self, match_id: &str) -> Result<MatchDto, String> {
        let url = format!("{}/lol/match/v5/matches/{}", REGIONAL_URL, match_id);
        self.get(&url).await
    }

    pub async fn get_match_timeline(&self, match_id: &str) -> Result<MatchTimelineDto, String> {
        let url = format!("{}/lol/match/v5/matches/{}/timeline", REGIONAL_URL, match_id);
        self.get(&url).await
    }

    pub async fn get_active_game(&self, puuid: &str) -> Result<Option<SpectatorGame>, String> {
        let summoner = self.get_summoner_by_puuid(puuid).await?;
        let Some(summoner_id) = summoner
            .id
            .filter(|id| !id.is_empty())
        else {
            return Ok(None);
        };
        let url = format!("{}/lol/spectator/v5/active-games/by-summoner/{}", PLATFORM_URL, summoner_id);
        self.get(&url).await.map(Some)
    }

    pub async fn get_matches_parallel(&self, match_ids: &[String], batch_size: usize) -> Vec<MatchDto> {
        let mut results = Vec::new();
        for chunk in match_ids.chunks(batch_size) {
            let futures: Vec<_> = chunk.iter().map(|id| self.get_match(id)).collect();
            let chunk_results = futures::future::join_all(futures).await;
            for r in chunk_results {
                match r {
                    Ok(m) => results.push(m),
                    Err(e) => log::warn!("Failed to fetch match: {}", e),
                }
            }
        }
        results
    }

    // ── Unthrottled methods for live-game enrichment ────────────────────────
    // These bypass the rate limiter to avoid blocking the champ select UI.
    // They still handle Riot 429 responses via retry, but don't wait in queue
    // behind match-history fetches or other background work.

    pub async fn get_account_by_riot_id_fast(&self, game_name: &str, tag_line: &str) -> Result<RiotAccount, String> {
        let url = format!("{}/riot/account/v1/accounts/by-riot-id/{}/{}", REGIONAL_URL, game_name, tag_line);
        self.get_raw(&url).await
    }

    pub async fn get_league_entries_fast(&self, summoner_id: &str) -> Result<Vec<LeagueEntry>, String> {
        let url = format!("{}/lol/league/v4/entries/by-summoner/{}", PLATFORM_URL, summoner_id);
        self.get_raw(&url).await
    }

    pub async fn get_league_entries_by_puuid_fast(&self, puuid: &str) -> Result<Vec<LeagueEntry>, String> {
        let url = format!("{}/lol/league/v4/entries/by-puuid/{}", PLATFORM_URL, puuid);
        self.get_raw(&url).await
    }

    pub async fn get_active_game_fast(&self, puuid: &str) -> Result<Option<SpectatorGame>, String> {
        let url_summoner = format!("{}/lol/summoner/v4/summoners/by-puuid/{}", PLATFORM_URL, puuid);
        let summoner: Summoner = self.get_raw(&url_summoner).await?;
        let Some(summoner_id) = summoner
            .id
            .filter(|id| !id.is_empty())
        else {
            return Ok(None);
        };
        let url = format!("{}/lol/spectator/v5/active-games/by-summoner/{}", PLATFORM_URL, summoner_id);
        self.get_raw(&url).await.map(Some)
    }
}
