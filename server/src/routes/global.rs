use axum::extract::State;
use axum::Json;
use std::sync::Arc;
use leagueeye_shared::models::*;
use crate::AppState;

// GET /api/global/dashboard
pub async fn get_global_dashboard(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GlobalDashboardData>, String> {
    let data = state.db.get_global_dashboard_data().await
        .map_err(|e| e.to_string())?;
    Ok(Json(data))
}
