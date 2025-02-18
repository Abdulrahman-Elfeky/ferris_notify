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
use crate::startup::AppState;

#[tracing::instrument(name = "Adding a new subscriber.", level = "debug",
    skip(state, form),
    fields(subscriber_email=%form.email,subscriber_name=%form.name))]
pub async fn subscribe(
    State(state): State<AppState>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let pool = state.pg_pool;
    match insert_subscriber(&pool, &form).await {
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
    skip(pool, form)
)]
async fn insert_subscriber(pool: &Pool<Postgres>, form: &FormData) -> sqlx::Result<(), Error> {
    query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1,$2,$3,$4) "#,
        Uuid::new_v4(),
        form.email,
        form.name,
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
