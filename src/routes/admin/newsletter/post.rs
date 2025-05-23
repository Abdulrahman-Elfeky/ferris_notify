use anyhow::Context;
use axum::{
    extract::State,
    http::{header::WWW_AUTHENTICATE, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Extension, Form,
};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, instrument, warn};

use crate::{
    authentication::UserId,
    domain::{SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    routes::admin::dashboard::get_username,
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
    #[error("Authentication Faild {0}")]
    AuthError(#[source] anyhow::Error),
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::UnexpectedError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "something went wrong.").into_response()
            }
            Self::AuthError(_) => {
                let mut headers = HeaderMap::new();
                headers.insert(
                    WWW_AUTHENTICATE,
                    HeaderValue::from_static(r#"Basic realm="publish""#),
                );
                (StatusCode::UNAUTHORIZED, headers, "Authentication Failed").into_response()
            }
        }
    }
}

#[instrument(
    name = "Publish a newsletter issue",
    skip_all,
    level = "debug",
    fields(username=tracing::field::Empty,user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    Extension(user_id): Extension<UserId>,
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    Form(body): Form<BodyData>,
) -> Result<impl IntoResponse, PublishError> {
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));
    let username = get_username(&pool, *user_id)
        .await
        .map_err(|e| PublishError::UnexpectedError(e))?;

    tracing::Span::current().record("username", tracing::field::display(&username));

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
