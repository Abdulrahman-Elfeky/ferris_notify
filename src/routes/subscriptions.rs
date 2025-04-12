use std::sync::Arc;

use anyhow::Context;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Form};
use chrono::Utc;
use rand::{distr::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::BaseUrl,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct FormData {
    name: String,
    email: String,
}

#[derive(thiserror::Error, Debug)]
pub enum SubscribeError {
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),

    #[error("{0}")]
    ValidationError(String),
}

impl IntoResponse for SubscribeError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Unexpected(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong.".into(),
            ),
            Self::ValidationError(e) => (StatusCode::BAD_REQUEST, e),
        }
        .into_response()
    }
}

impl From<String> for SubscribeError {
    fn from(e: String) -> Self {
        SubscribeError::ValidationError(e)
    }
}

#[tracing::instrument(name = "Adding a new subscriber.", level = "debug",
    skip(pool, form,email_client,base_url),
    fields(subscriber_email=%form.email,subscriber_name=%form.name))]
pub async fn subscribe(
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    State(base_url): State<Arc<BaseUrl>>,
    Form(form): Form<FormData>,
) -> Result<impl IntoResponse, SubscribeError> {
    let new_subscriber = form.try_into()?;

    let mut transaction = pool.begin().await.context("Failed to begin transaction.")?;

    //let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
    //    Ok(subscriber_id) => subscriber_id,
    //    Err(_) => return Ok(StatusCode::INTERNAL_SERVER_ERROR.into()),
    //};
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert subscriber.")?;

    let subscription_token = generate_subscription_token();

    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store token.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit transaction.")?;

    send_email(
        &email_client,
        &new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .context("Failed to send email.")?;

    info!("New subscriber details have been saved.",);

    Ok(StatusCode::OK)
}

#[tracing::instrument(
    name = "Sending a confirmation email.",
    level = "debug",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
async fn send_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?confirmation_token={}",
        base_url, subscription_token
    );

    email_client
        .send_email(
            &new_subscriber.email,
            &new_subscriber.name,
            "Welcome",
            &format!(
                "Welcome to our newsletter <br>
                        Click <a href={}>Here</a> to confirm your email.",
                confirmation_link
            ),
        )
        .await
}

#[tracing::instrument(
    name = "Saving a new subscriber details in the database.",
    level = "debug",
    skip(transaction, new_subscriber)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> sqlx::Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1,$2,$3,$4,'pending_confirmation') "#,
        id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(transaction.as_mut())
    .await?;

    Ok(id)
}

#[tracing::instrument(
    name = "Saving a new subscription token in the database.",
    level = "debug",
    skip(transaction, subscriber_id, subscription_token)
)]
async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO subscription_tokens (subscriber_id, subscription_token) VALUES ($1,$2)",
        subscriber_id,
        subscription_token
    )
    .execute(transaction.as_mut())
    .await?;

    Ok(())
}

fn generate_subscription_token() -> String {
    let mut rng = rand::rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;
        Ok(Self { name, email })
    }
}
