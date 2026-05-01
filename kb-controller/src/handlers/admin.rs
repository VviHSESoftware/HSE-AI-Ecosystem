use crate::schemas::{CreateSystemReq, CreateSystemRes};
use crate::{state::KbControllerState};
use axum::extract::{Json, State};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use common_infra::AppError;

#[utoipa::path(post, path = "/api/v1/admin/createSystem", request_body = CreateSystemReq, responses((status = 200, body = CreateSystemRes)), security(("bearerAuth" = [])))]
pub async fn create_system(
    State(state): State<Arc<KbControllerState>>,
    Json(req): Json<CreateSystemReq>
) -> Result<Json<CreateSystemRes>, AppError> {
    let raw_token = uuid::Uuid::new_v4().to_string();

    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    let hash = hex::encode(hasher.finalize());

    let mut tx = state.db.begin().await?;

    let row = sqlx::query("INSERT INTO modules (name, type) VALUES ($1, 'System') RETURNING id")
        .bind(&req.name).fetch_one(&mut *tx).await?;
    let system_id: i32 = sqlx::Row::get(&row, "id");

    sqlx::query("UPDATE modules SET path = $1 WHERE id = $2")
        .bind(format!("/{}", system_id)).bind(system_id).execute(&mut *tx).await?;

    sqlx::query("INSERT INTO module_system (module_id, token_hash) VALUES ($1, $2)")
        .bind(system_id).bind(hash).execute(&mut *tx).await?;

    tx.commit().await?;

    Ok(Json(CreateSystemRes {
        system_id: system_id,
        system_token: raw_token,
        note: "Save this token. It won't be shown again.".to_string()
    }))
}