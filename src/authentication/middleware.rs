use std::{fmt::Display, ops::Deref};

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use reqwest::StatusCode;
use uuid::Uuid;

use crate::session_state::TypedSession;

#[derive(Clone)]
pub struct UserId(Uuid);

impl Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for UserId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn reject_anonymous_users(
    session: TypedSession,
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    match session
        .get_user_id()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
    {
        Some(user_id) => {
            req.extensions_mut().insert(UserId(user_id));
            Ok(next.run(req).await)
        }
        None => Ok(Redirect::to("/login").into_response()),
    }
}
