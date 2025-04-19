use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    extract::{FromRequestParts, Json, State},
    http::{header::WWW_AUTHENTICATE, request::Parts, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, instrument, warn};
use uuid::Uuid;

use crate::{
    domain::{SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    telemetry::spawn_blocking_with_tracing,
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

pub struct Credentials {
    username: String,
    password: SecretString,
}

impl<S> FromRequestParts<S> for Credentials
where
    S: Send + Sync,
{
    type Rejection = PublishError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .extract::<TypedHeader<Authorization<Basic>>>()
            .await
            .context("Invalid authorization header")
            .map_err(PublishError::AuthError)?;

        let username = auth_header.username().to_string();
        let password = auth_header.password().to_string();

        Ok(Credentials {
            username,
            password: SecretString::from(password),
        })
    }
}

#[instrument(
    name = "Publish a newsletter issue",
    skip_all,
    level = "debug",
    fields(username=tracing::field::Empty,user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    credentials: Credentials,
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    Json(body): Json<BodyData>,
) -> Result<impl IntoResponse, PublishError> {
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    let user_id = validate_credentials(&pool, credentials)
        .await
        .map_err(|e| {
            error!(error = tracing::field::display(&e), "The error is ");
            e
        })?;

    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

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

#[tracing::instrument("Validate credentials", skip_all, level = "debug")]
async fn validate_credentials(
    pool: &PgPool,
    credentials: Credentials,
) -> Result<Uuid, PublishError> {
    let mut user_id = None;
    let mut expected_password_hash = SecretString::from(
        "$argon2id$v=19$m=15000,t=2,p=1$\
            gZiV/M1gPc22ElAH/Jh1Hw$\
            CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno",
    );
    if let Some((stored_user_id, stored_password_hash)) = get_stored_credentials(pool, &credentials)
        .await
        .map_err(PublishError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(credentials.password, expected_password_hash)
    })
    .await
    .context("Faild to spawn a blocking task.")
    .map_err(PublishError::UnexpectedError)??;

    user_id.ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username.")))
}

#[tracing::instrument("Get stored credentials", skip_all, level = "debug")]
async fn get_stored_credentials(
    pool: &PgPool,
    credentials: &Credentials,
) -> Result<Option<(Uuid, SecretString)>, anyhow::Error> {
    let row = sqlx::query!(
        "SELECT user_id, password_hash FROM users where username = $1  ;",
        credentials.username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")?
    .map(|r| (r.user_id, SecretString::from(r.password_hash)));

    Ok(row)
}

#[tracing::instrument("Verify password hash", level = "debug", skip_all)]
fn verify_password_hash(
    password: SecretString,
    expected_password_hash: SecretString,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(&expected_password_hash.expose_secret())
        .context("Faild to parse hash in PHC string format.")
        .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &expected_password_hash)
        .context("Invalid passowrd")
        .map_err(PublishError::AuthError)
}
