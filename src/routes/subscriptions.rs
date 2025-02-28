use axum::{extract::State, http::StatusCode, response::IntoResponse, Form};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{query, Error, Pool, Postgres};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct FormData {
    name: String,
    email: String,
}
use crate::{
    domain::{NewSubscriber, SubscriberName},
    startup::AppState,
};

#[tracing::instrument(name = "Adding a new subscriber.", level = "debug",
    skip(state, form),
    fields(subscriber_email=%form.email,subscriber_name=%form.name))]
pub async fn subscribe(
    State(state): State<AppState>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let pool = state.pg_pool;

    let name = match SubscriberName::parse(form.name) {
        Ok(name) => name,
        Err(_) => {
            return StatusCode::BAD_REQUEST;
        }
    };

    let new_subscriber = NewSubscriber {
        email: form.email,
        name,
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
        new_subscriber.email,
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
