use anyhow::Context;
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Algorithm, Argon2, Params, PasswordHash, PasswordVerifier, Version,
};
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use reqwest::header::WWW_AUTHENTICATE;
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;
use uuid::Uuid;

use crate::telemetry::spawn_blocking_with_tracing;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid Credentials.")]
    InvalidCredentials(#[source] anyhow::Error),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AuthError::UnexpectedError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "something went wrong.").into_response()
            }
            AuthError::InvalidCredentials(_) => {
                let mut headers = HeaderMap::new();
                headers.insert(
                    WWW_AUTHENTICATE,
                    HeaderValue::from_static(r#"Basic realm="publish""#),
                );

                (StatusCode::UNAUTHORIZED, headers, "Invalid credentials.").into_response()
            }
        }
    }
}

pub struct Credentials {
    pub username: String,
    pub password: SecretString,
}

impl<S> FromRequestParts<S> for Credentials
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .extract::<TypedHeader<Authorization<Basic>>>()
            .await
            .context("Invalid authorization header")
            .map_err(AuthError::InvalidCredentials)?;

        let username = auth_header.username().to_string();
        let password = auth_header.password().to_string();

        Ok(Credentials {
            username,
            password: SecretString::from(password),
        })
    }
}
#[tracing::instrument("Validate credentials", skip_all, level = "debug")]
pub async fn validate_credentials(
    pool: &PgPool,
    credentials: Credentials,
) -> Result<Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = SecretString::from(
        "$argon2id$v=19$m=15000,t=2,p=1$\
            gZiV/M1gPc22ElAH/Jh1Hw$\
            CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno",
    );
    if let Some((stored_user_id, stored_password_hash)) = get_stored_credentials(pool, &credentials)
        .await
        .map_err(AuthError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(credentials.password, expected_password_hash)
    })
    .await
    .context("Faild to spawn a blocking task.")
    .map_err(AuthError::UnexpectedError)??;

    user_id.ok_or_else(|| AuthError::InvalidCredentials(anyhow::anyhow!("Unknown username.")))
}

#[tracing::instrument("Get stored credentials", skip_all, level = "debug")]
async fn get_stored_credentials(
    pool: &PgPool,
    credentials: &Credentials,
) -> Result<Option<(Uuid, SecretString)>, anyhow::Error> {
    let row = sqlx::query!(
        "SELECT user_id, password_hash FROM users where username = $1  ;",
        credentials.username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")?
    .map(|r| (r.user_id, SecretString::from(r.password_hash)));

    Ok(row)
}

#[tracing::instrument("Verify password hash", level = "debug", skip_all)]
fn verify_password_hash(
    password: SecretString,
    expected_password_hash: SecretString,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(&expected_password_hash.expose_secret())
        .context("Faild to parse hash in PHC string format.")
        .map_err(AuthError::UnexpectedError)?;

    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &expected_password_hash)
        .context("Invalid passowrd")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Change password", skip(password, pool))]
pub async fn change_password(
    user_id: &Uuid,
    password: SecretString,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await?
        .context("Failed to hash password")?;

    sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $1 WHERE user_id = $2 "#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to change user's password in the database.")?;

    Ok(())
}

fn compute_password_hash(password: SecretString) -> Result<SecretString, anyhow::Error> {
    let salt = SaltString::generate(rand2::thread_rng());
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();

    Ok(SecretString::from(password_hash))
}
