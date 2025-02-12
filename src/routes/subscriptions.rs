use axum::{extract::State, http::StatusCode, response::IntoResponse, Form};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::query;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct FormData {
    name: String,
    email: String,
}
use crate::startup::AppState;
pub async fn subscribe(
    State(state): State<AppState>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let pool = state.pg_pool;
    query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1,$2,$3,$4) "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(&pool)
    .await
    .ok();
    StatusCode::OK
}
