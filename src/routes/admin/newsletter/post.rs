use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Extension, Form,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{instrument, warn};

use crate::{
    authentication::UserId,
    domain::{SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    errors::ApiError,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    routes::admin::dashboard::get_username,
};

#[derive(Deserialize)]
pub struct FormData {
    title: String,
    html_content: String,
    idempotency_key: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
    name: SubscriberName,
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
    Form(form): Form<FormData>,
) -> Result<Response, ApiError> {
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));
    let username = get_username(&pool, *user_id).await?;
    tracing::Span::current().record("username", tracing::field::display(&username));
    let FormData {
        title,
        html_content,
        idempotency_key,
    } = form;

    let idempotency: IdempotencyKey = idempotency_key.try_into().map_err(ApiError::Validation)?;

    let transaction = match try_processing(&pool, *user_id, &idempotency).await? {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(res) => {
            return Ok(res);
        }
    };

    let confirmed_subscribers = get_confirmed_subscribers(&pool).await?;

    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &subscriber.name, &title, &html_content)
                    .await?
            }
            Err(_) => {
                warn!("Skipping confirmed subscriber. Their stored contact details are invalid.",);
            }
        }
    }
    let response = (
        CookieJar::new().add(Cookie::new(
            "_flash",
            "The newsletter issue has been published!",
        )),
        Redirect::to("/admin/newsletters"),
    )
        .into_response();

    let response = save_response(transaction, *user_id, &idempotency, response).await?;

    Ok(response)
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
