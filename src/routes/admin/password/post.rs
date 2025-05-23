use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Extension, Form,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use reqwest::StatusCode;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials, UserId},
    routes::admin::dashboard::get_username,
};

#[derive(Deserialize)]
pub struct FormData {
    current_password: SecretString,
    new_password: SecretString,
    new_password_check: SecretString,
}

pub async fn change_password(
    jar: CookieJar,
    Extension(user_id): Extension<UserId>,
    State(pool): State<PgPool>,
    Form(form): Form<FormData>,
) -> Result<Response, Response> {
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        return Ok((
            jar.add(Cookie::new(
                "_flash",
                "You entered two different new passwords - the field values must match.",
            )),
            Redirect::to("/admin/password"),
        )
            .into_response());
    }

    let username = get_username(&pool, *user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let credentials = Credentials {
        username,
        password: form.current_password,
    };
    if let Err(e) = validate_credentials(&pool, credentials).await {
        match e {
            AuthError::InvalidCredentials(_) => {
                return Ok((
                    jar.add(Cookie::new("_flash", "The current password is incorrect.")),
                    Redirect::to("/admin/password"),
                )
                    .into_response())
            }

            AuthError::UnexpectedError(_) => {
                return Err(e.into_response());
            }
        }
    }

    crate::authentication::change_password(&user_id, form.new_password, &pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    Ok((
        jar.add(Cookie::new("_flash", "Your password has been changed.")),
        Redirect::to("/admin/password"),
    )
        .into_response())
}
