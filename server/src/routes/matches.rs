use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;

use leagueeye_shared::models::*;
use crate::AppState;

// GET /api/matches/{match_id}
pub async fn get_match_detail(
    State(state): State<Arc<AppState>>,
    Path(match_id): Path<String>,
) -> Result<Json<MatchDetail>, String> {
    // Try cache first
    if let Some(detail) = state.db.get_match_detail(&match_id).await.map_err(|e| e.to_string())? {
        return Ok(Json(detail));
    }

    // Cache miss: fetch from Riot API
    let dto = state.riot_api.get_match(&match_id).await?;
    let parts = dto_to_participants(&dto);
    let _ = state.db.save_match_participants(&dto.metadata.match_id, &parts).await;

    // Also save the match summary for the first participant found in our DB
    // This ensures game_duration/game_creation are available for the detail query

    let detail = state.db.get_match_detail(&match_id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Детали матча не найдены после загрузки".to_string())?;

    Ok(Json(detail))
}
