use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::instrument;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    session_state::TypedSession,
};

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("something went wrong")]
    UnexpectedError(#[from] anyhow::Error),

    #[error("Invalid credentials")]
    AuthErr(#[source] anyhow::Error),
}

impl From<AuthError> for LoginError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::InvalidCredentials(error) => LoginError::AuthErr(error),
            AuthError::UnexpectedError(error) => LoginError::UnexpectedError(error),
        }
    }
}

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        match self {
            Self::UnexpectedError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "something went wrong:(").into_response()
            }
            //Self::AuthErr(_) => (StatusCode::UNAUTHORIZED, "Invalid credentials."),
            Self::AuthErr(_) => {
                let jar = CookieJar::new();

                (
                    jar.add(Cookie::new("_flash", "Authentication failed")),
                    Redirect::to("/login"),
                )
            }
            .into_response(),
        }
        .into_response()
    }
}

#[instrument(skip_all,fields(username=tracing::field::Empty,user_id=tracing::field::Empty))]
pub async fn login(
    State(pool): State<PgPool>,
    session: TypedSession,
    Form(form): Form<FormData>,
) -> Result<impl IntoResponse, LoginError> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    let user_id = validate_credentials(&pool, credentials).await?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    session
        .cycle_id()
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    session
        .insert_user_id(user_id)
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    Ok(Redirect::to("/admin/dashboard"))
}
