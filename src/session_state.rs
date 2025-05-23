use axum::{extract::FromRequestParts, http::request::Parts};
use reqwest::StatusCode;
use tower_sessions::{session, Session};
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    pub async fn cycle_id(&self) -> Result<(), session::Error> {
        self.0.cycle_id().await
    }
    pub async fn get_user_id(&self) -> Result<Option<Uuid>, session::Error> {
        self.0.get(Self::USER_ID_KEY).await
    }

    pub async fn insert_user_id(&self, uuid: Uuid) -> Result<(), session::Error> {
        self.0.insert(Self::USER_ID_KEY, uuid).await
    }

    pub async fn log_out(&self) -> Result<(), session::Error> {
        self.0.flush().await
    }
}

impl<S> FromRequestParts<S> for TypedSession
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state).await?;
        Ok(Self(session))
    }
}
