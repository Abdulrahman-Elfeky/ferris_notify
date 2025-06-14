use std::{collections::HashMap, u16};

use anyhow::Context;
use axum::{
    body::{self, Body},
    response::Response,
};
use reqwest::StatusCode;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::IdempotencyKey;

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(Response),
}

pub async fn try_processing(
    pool: &PgPool,
    user_id: Uuid,
    idempotency_key: &IdempotencyKey,
) -> Result<NextAction, anyhow::Error> {
    let mut transaction = pool.begin().await?;

    let n_inserted_rows = sqlx::query!(
        r#"
        INSERT INTO idempotency
        (user_id,
        idempotency_key,
        created_at) VALUES
        ($1, $2, now())
        ON CONFLICT DO NOTHING;"#,
        user_id,
        idempotency_key.as_ref(),
    )
    .execute(transaction.as_mut())
    .await?
    .rows_affected();

    if n_inserted_rows > 0 {
        return Ok(NextAction::StartProcessing(transaction));
    } else {
        let saved_response = get_saved_response(pool, user_id, idempotency_key)
            .await?
            .ok_or_else(|| anyhow::anyhow!("We expected a saved response, we didn't find it."))?;
        return Ok(NextAction::ReturnSavedResponse(saved_response));
    }
}

pub async fn get_saved_response(
    pool: &PgPool,
    user_id: Uuid,
    idempotency_key: &IdempotencyKey,
) -> Result<Option<Response>, anyhow::Error> {
    let response = sqlx::query_as!(
        ResponseRecord,
        r#"SELECT response_status_code as "response_status_code!",
        response_headers as "response_headers!",
        response_body as "response_body!",
        created_at
        FROM idempotency 
        WHERE user_id = $1 and idempotency_key = $2 "#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(pool)
    .await?;

    response.map(|record| record.to_response()).transpose()
}

pub async fn save_response(
    mut transaction: Transaction<'static, Postgres>,
    user_id: Uuid,
    idempotency_key: &IdempotencyKey,
    response: Response,
) -> Result<Response, anyhow::Error> {
    let status_code = response.status().as_u16() as i16;

    let header_map = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect::<HashMap<_, _>>();
    let headers_json = serde_json::to_value(&header_map)?;

    let body_bytes = body::to_bytes(response.into_body(), usize::MAX).await?;
    let body = body_bytes.to_vec();

    sqlx::query!(
        r#"
            UPDATE idempotency SET 
            response_status_code = $3, 
            response_headers = $4, 
            response_body = $5 
            WHERE user_id = $1 AND 
            idempotency_key = $2
            "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers_json,
        body,
    )
    .execute(transaction.as_mut())
    .await?;

    transaction.commit().await?;

    let mut res = Response::builder().status(status_code as u16);

    for (k, v) in header_map {
        res = res.header(k, v);
    }

    let res = res.body(Body::from(body))?;

    Ok(res)
}

#[allow(unused)]
struct ResponseRecord {
    response_status_code: i16,
    response_headers: serde_json::Value,
    response_body: Vec<u8>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl ResponseRecord {
    fn to_response(self) -> Result<Response, anyhow::Error> {
        let status = StatusCode::from_u16(self.response_status_code as u16)?;

        let mut response_builder = Response::builder().status(status);

        if let Ok(header_map) =
            serde_json::from_value::<HashMap<String, String>>(self.response_headers)
        {
            for (key, value) in header_map {
                response_builder = response_builder.header(key, value);
            }
        }

        let body = Body::from(self.response_body);

        response_builder.body(body).context("")
    }
}
