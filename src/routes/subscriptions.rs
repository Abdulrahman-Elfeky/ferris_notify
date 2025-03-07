use axum::{extract::State, http::StatusCode, response::IntoResponse, Form};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{query, Error, PgPool, Pool, Postgres};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct FormData {
    name: String,
    email: String,
}
use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

#[tracing::instrument(name = "Adding a new subscriber.", level = "debug",
    skip(pool, form),
    fields(subscriber_email=%form.email,subscriber_name=%form.name))]
pub async fn subscribe(
    State(pool): State<PgPool>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let new_subscriber = match form.try_into() {
        Ok(new_subscriber) => new_subscriber,
        Err(_) => {
            return StatusCode::BAD_REQUEST;
        }
    };

    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(_) => {
            info!("New subscriber details have been saved.",);
            StatusCode::OK
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(
    name = "Saving a new subscriber details in the database.",
    level = "debug",
    skip(pool, new_subscriber)
)]
async fn insert_subscriber(
    pool: &Pool<Postgres>,
    new_subscriber: &NewSubscriber,
) -> sqlx::Result<(), Error> {
    query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1,$2,$3,$4) "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        error!("Failed to execute query {e:?}");
        e
    })?;

    Ok(())
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;
        Ok(Self { name, email })
    }
}
