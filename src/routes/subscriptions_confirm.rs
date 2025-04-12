use anyhow::Context;
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

#[derive(Debug, thiserror::Error)]
pub enum ConfirmationError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),

    #[error("There is no subscriber associated with the given token.")]
    UnknownToken,
}

impl IntoResponse for ConfirmationError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::UnknownToken => (StatusCode::UNAUTHORIZED, "Invalid token."),
            Self::UnexpectedError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "something went wrong.")
            }
        }
        .into_response()
    }
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(
    Query(parameters): Query<Parameters>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, ConfirmationError> {
    let id = get_subscriber_id_from_token(&pool, &parameters.confirmation_token)
        .await
        .context("Failed to retrieve the subscriber associated with the given token.")?
        .ok_or(ConfirmationError::UnknownToken)?;

    confirm_subscriber(&pool, id)
        .await
        .context("Failed to confirm the subscriber.")?;

    Ok(StatusCode::OK)
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
