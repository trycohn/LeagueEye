use axum::{Router, routing::{get, post}};
use std::net::SocketAddr;
use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{CorsLayer, Any};

mod riot_api;
mod db;
mod routes;

#[derive(Debug, Clone)]
pub struct AiCoachConfig {
    pub provider: AiCoachProvider,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    // OpenRouter-only (optional attribution headers)
    pub openrouter_http_referer: Option<String>,
    pub openrouter_title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiCoachProvider {
    Anthropic,
    OpenRouter,
    DeepSeek,
}

pub struct AppState {
    pub riot_api: riot_api::RiotApiClient,
    pub db: db::Db,
    pub ai_coach_config: Option<AiCoachConfig>,
}

async fn health() -> &'static str {
    "LeagueEye server OK"
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    env_logger::init();

    let api_key = std::env::var("RIOT_API_KEY")
        .expect("RIOT_API_KEY must be set");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://leagueeye:leagueeye@localhost/leagueeye".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    log::info!("Database connected and migrated");

    // AI Coach config (optional — server works without it)
    let ai_coach_config = load_ai_coach_config();

    if ai_coach_config.is_none() {
        log::warn!("AI Coach disabled (no provider configured)");
    }

    let state = Arc::new(AppState {
        riot_api: riot_api::RiotApiClient::new(api_key),
        db: db::Db::new(pool),
        ai_coach_config,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        // Global endpoints
        .route("/api/global/dashboard", get(routes::global::get_global_dashboard))
        // Player endpoints
        .route("/api/players/{game_name}/{tag_line}", get(routes::players::search_player))
        .route("/api/players/{puuid}/mastery", get(routes::players::get_mastery))
        .route("/api/players/{puuid}/matches", get(routes::players::get_matches_and_stats))
        // Match endpoints
        .route("/api/matches/{match_id}", get(routes::matches::get_match_detail))
        // Live game enrichment
        .route("/api/live/enrich", post(routes::live::enrich_live_game))
        // AI Coach streaming
        .route("/api/coach/stream", post(routes::coach::stream_coach))
        .layer(cors)
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            log::info!("LeagueEye server listening on {}", addr);
            listener
        }
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            log::error!(
                "Failed to start LeagueEye server on {}: address already in use. Another process is already listening on this port.",
                addr
            );
            std::process::exit(98);
        }
        Err(error) => {
            log::error!("Failed to bind LeagueEye server to {}: {}", addr, error);
            std::process::exit(1);
        }
    };

    if let Err(error) = axum::serve(listener, app).await {
        log::error!("LeagueEye server stopped with error: {}", error);
        std::process::exit(1);
    }
}

fn load_ai_coach_config() -> Option<AiCoachConfig> {
    let provider_env = std::env::var("AI_COACH_PROVIDER").ok()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());

    let anthropic_key = std::env::var("ANTHROPIC_AUTH_TOKEN").ok()
        .filter(|k| !k.trim().is_empty());
    let openrouter_key = std::env::var("OPENROUTER_API_KEY").ok()
        .filter(|k| !k.trim().is_empty());
    let deepseek_key = std::env::var("DEEPSEEK_API_KEY").ok()
        .filter(|k| !k.trim().is_empty());

    let provider = match provider_env.as_deref() {
        Some("anthropic") => Some(AiCoachProvider::Anthropic),
        Some("openrouter") => Some(AiCoachProvider::OpenRouter),
        Some("deepseek") => Some(AiCoachProvider::DeepSeek),
        Some(other) => {
            log::warn!(
                "Unknown AI_COACH_PROVIDER='{}' (use 'anthropic', 'openrouter' or 'deepseek')",
                other
            );
            None
        }
        None => {
            // Backwards-compatible default: if Anthropic key is set, keep using Anthropic.
            if anthropic_key.is_some() {
                Some(AiCoachProvider::Anthropic)
            } else if openrouter_key.is_some() {
                Some(AiCoachProvider::OpenRouter)
            } else if deepseek_key.is_some() {
                Some(AiCoachProvider::DeepSeek)
            } else {
                None
            }
        }
    }?;

    match provider {
        AiCoachProvider::Anthropic => {
            let api_key = anthropic_key?;
            let base_url = std::env::var("ANTHROPIC_BASE_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com".to_string());
            let model = std::env::var("ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
            let max_tokens = std::env::var("AI_COACH_MAX_TOKENS").ok()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(1024);
            log::info!("AI Coach enabled (provider: Anthropic, model: {})", model);
            Some(AiCoachConfig {
                provider,
                api_key,
                base_url,
                model,
                max_tokens,
                openrouter_http_referer: None,
                openrouter_title: None,
            })
        }
        AiCoachProvider::OpenRouter => {
            let api_key = openrouter_key?;
            let base_url = std::env::var("OPENROUTER_BASE_URL")
                .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());
            let model = std::env::var("OPENROUTER_MODEL")
                .unwrap_or_else(|_| "openai/gpt-4o-mini".to_string());
            let max_tokens = std::env::var("AI_COACH_MAX_TOKENS").ok()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(1024);
            let http_referer = std::env::var("OPENROUTER_HTTP_REFERER").ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let title = std::env::var("OPENROUTER_TITLE").ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            log::info!("AI Coach enabled (provider: OpenRouter, model: {})", model);
            Some(AiCoachConfig {
                provider,
                api_key,
                base_url,
                model,
                max_tokens,
                openrouter_http_referer: http_referer,
                openrouter_title: title,
            })
        }
        AiCoachProvider::DeepSeek => {
            let api_key = deepseek_key?;
            let base_url = std::env::var("DEEPSEEK_BASE_URL")
                .unwrap_or_else(|_| "https://api.deepseek.com".to_string());
            let model = std::env::var("DEEPSEEK_MODEL")
                .unwrap_or_else(|_| "deepseek-chat".to_string());
            let max_tokens = std::env::var("AI_COACH_MAX_TOKENS").ok()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(1024);
            log::info!("AI Coach enabled (provider: DeepSeek, model: {})", model);
            Some(AiCoachConfig {
                provider,
                api_key,
                base_url,
                model,
                max_tokens,
                openrouter_http_referer: None,
                openrouter_title: None,
            })
        }
    }
}
