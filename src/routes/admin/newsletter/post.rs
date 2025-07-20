use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Extension, Form,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    authentication::UserId,
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

#[instrument(
    name = "Publish a newsletter issue",
    skip_all,
    level = "debug",
    fields(username=tracing::field::Empty,user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    Extension(user_id): Extension<UserId>,
    State(pool): State<PgPool>,
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

    let mut transaction = match try_processing(&pool, *user_id, &idempotency).await? {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(res) => {
            return Ok(res);
        }
    };

    let newsletter_id = insert_newsletter_issue(&mut transaction, title, html_content).await?;
    enque_delivery_tasks(&mut transaction, newsletter_id).await?;

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

#[instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'static, Postgres>,
    title: String,
    html_content: String,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_id = Uuid::new_v4();
    sqlx::query!(
        r#"
    INSERT INTO newsletter_issues(
    newsletter_id,
    title,
    html_content,
    published_at
    ) values
    ($1, $2, $3, now())
        "#,
        newsletter_id,
        title,
        html_content
    )
    .execute(transaction.as_mut())
    .await?;

    Ok(newsletter_id)
}

#[instrument(skip_all)]
async fn enque_delivery_tasks(
    transaction: &mut Transaction<'static, Postgres>,
    newsletter_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
    INSERT INTO issue_delivery_queue(
        newsletter_id,
        subscriber_email,
        name
    ) SELECT $1 , email, name FROM subscriptions WHERE status = 'confirmed'
        "#,
        newsletter_id
    )
    .execute(transaction.as_mut())
    .await?;

    Ok(())
}
