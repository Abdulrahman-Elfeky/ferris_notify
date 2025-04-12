use anyhow::Context;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{instrument, warn};

use crate::{
    domain::{SubscriberEmail, SubscriberName},
    email_client::EmailClient,
};

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    html_content: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
    name: SubscriberName,
}

#[derive(thiserror::Error, Debug)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::UnexpectedError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "something went wrong.")
            }
        }
        .into_response()
    }
}

#[instrument(
    name = "Publishing a newsletter",
    skip(body, pool, email_client),
    level = "debug",
    fields(title=%body.title)
)]
pub async fn publish_newsletter(
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    Json(body): Json<BodyData>,
) -> Result<impl IntoResponse, PublishError> {
    let confirmed_subscribers = get_confirmed_subscribers(&pool).await?;

    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(subscriber) => email_client
                .send_email(
                    &subscriber.email,
                    &subscriber.name,
                    &body.title,
                    &body.html_content,
                )
                .await
                .with_context(|| format!("Cann't send email to {}", subscriber.email))?,
            Err(_) => {
                warn!("Skipping confirmed subscriber. Their stored contact details are invalid.",);
            }
        }
    }
    Ok(StatusCode::OK)
}

#[instrument(name = "Get confirmed subscribers", skip(pool), level = "debug")]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers =
        sqlx::query!("SELECT email, name FROM subscriptions WHERE status = 'confirmed'")
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|r| {
                match (
                    SubscriberEmail::parse(r.email),
                    SubscriberName::parse(r.name),
                ) {
                    (Ok(email), Ok(name)) => Ok(ConfirmedSubscriber { email, name }),
                    (_, _) => Err(anyhow::anyhow!("invalid email or name")),
                }
            })
            .collect();
    Ok(confirmed_subscribers)
}
