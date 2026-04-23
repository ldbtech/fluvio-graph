/// ---- GET /codebase/galaxy
/// 
use serde::Deserialize;

use axum::{http::StatusCode, Json};
use axum::extract::Query;

#[derive(Deserialize)]
pub struct GalaxyQuery {
    url: Option<String>,
    owner: Option<String>,
    repo: Option<String>,
}

pub async fn get_codebase_galaxy(
    Query(q): Query<GalaxyQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let url = if let Some(u) = q.url.filter(|s| !s.trim().is_empty()) {
        u
    }else {
        let o = q.owner.filter(|s| !s.trim().is_empty())
                      .ok_or((StatusCode::BAD_REQUEST, "pass url = or owner=&repo=".to_string()))?;
        let r = q.repo.filter(|s| !s.trim().is_empty())
                        .ok_or((StatusCode::BAD_REQUEST, "pass url = or owner=&repo=".to_string()))?;
        format!("{o}/{r}")
    };

    let join = tokio::task::spawn_blocking(move || {
        crate::ingestion_registry::codebase::tree::build_tree(&url)
    });

    let tree = join.await 
                   .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                   .map_err(|e| {
                    let status = match &e {
                        crate::ingestion_registry::codebase::tree::TreeError::NotCloned(_) => StatusCode::NOT_FOUND,
                        crate::ingestion_registry::codebase::tree::TreeError::InvalidUrl(_) => StatusCode::BAD_REQUEST,
                        crate::ingestion_registry::codebase::tree::TreeError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
                    };
                    (status, e.to_string())
                   })?;
    Ok(Json(serde_json::to_value(tree)
             .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?))
}