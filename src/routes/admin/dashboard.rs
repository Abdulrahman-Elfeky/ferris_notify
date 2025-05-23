use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Extension,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::authentication::UserId;

pub async fn admin_dashboard(
    Extension(user_id): Extension<UserId>,
    State(pool): State<PgPool>,
) -> Result<Response, Response> {
    let username = get_username(&pool, *user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    return Ok(Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta http-equiv="content-type" content="text/html; charset=utf-8">
<title>Admin dashboard</title>
</head>
<body>
<p>Welcome {username}!</p>
<p>Available actions:</p>
<ol>
<li><a href="/admin/password">Change password</a></li>
<li><a href="/admin/newsletters">Send newsletter issue</a></li>
<li>
<form name="logoutForm" action="/admin/logout" method="post">
<input type="submit" value="Logout">
</form>
</li>
</ol>
</body>
</html>"#
    ))
    .into_response());
}

pub async fn get_username(pool: &PgPool, user_id: Uuid) -> Result<String, anyhow::Error> {
    let row = sqlx::query!("SELECT username FROM users where user_id = $1 ", user_id)
        .fetch_one(pool)
        .await
        .context("Failed to perform a query to retrieve a username.")?;

    Ok(row.username)
}
