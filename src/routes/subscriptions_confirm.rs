use axum::{
    extract::{Query, State},
    http::status::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::error;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Parameters {
    confirmation_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(
    Query(parameters): Query<Parameters>,
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    let id = match get_subscriber_id_from_token(&pool, &parameters.confirmation_token).await {
        Ok(id) => id,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    match id {
        None => StatusCode::UNAUTHORIZED,
        Some(id) => {
            if confirm_subscriber(&pool, id).await.is_err() {
                StatusCode::INTERNAL_SERVER_ERROR
            } else {
                StatusCode::OK
            }
        }
    }
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(token, pool))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let record = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens where subscription_token = $1",
        token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        error!("Failed to execute query {:?}", e);
        e
    })?;

    Ok(record.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(id, pool))]
async fn confirm_subscriber(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE subscriptions SET status = 'confirmed' WHERE id = $1",
        id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        error!("Failed to execute query {:?}", e);
        e
    })?;

    Ok(())
}
