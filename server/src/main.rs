use axum::{Router, routing::{get, post}};
use std::net::SocketAddr;
use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{CorsLayer, Any};

mod riot_api;
mod db;
mod routes;

pub struct AppState {
    pub riot_api: riot_api::RiotApiClient,
    pub db: db::Db,
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

    let state = Arc::new(AppState {
        riot_api: riot_api::RiotApiClient::new(api_key),
        db: db::Db::new(pool),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        // Player endpoints
        .route("/api/players/{game_name}/{tag_line}", get(routes::players::search_player))
        .route("/api/players/{puuid}/mastery", get(routes::players::get_mastery))
        .route("/api/players/{puuid}/matches", get(routes::players::get_matches_and_stats))
        // Match endpoints
        .route("/api/matches/{match_id}", get(routes::matches::get_match_detail))
        // Live game enrichment
        .route("/api/live/enrich", post(routes::live::enrich_live_game))
        .layer(cors)
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("LeagueEye server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
