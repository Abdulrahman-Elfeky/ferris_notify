use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use reqwest::StatusCode;

use crate::session_state::TypedSession;

pub async fn log_out(session: TypedSession, jar: CookieJar) -> Result<Response, Response> {
    session
        .log_out()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    let mut flash_message = Cookie::new("_flash", "You have successfully logged out.");
    flash_message.set_path("/");

    Ok((jar.add(flash_message), Redirect::to("/login")).into_response())
}
