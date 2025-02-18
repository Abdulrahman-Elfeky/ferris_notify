use std::io;

use axum::{
    routing::{get, post},
    Router,
};

use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::{
    routes::{health_check, subscribe},
    telemetry::RequestIdSpan,
};

#[derive(Clone, Debug)]
pub struct AppState {
    pub pg_pool: PgPool,
}
pub async fn run(listener: TcpListener, pg_pool: PgPool) -> io::Result<()> {
    let router = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(AppState { pg_pool })
        .layer(TraceLayer::new_for_http().make_span_with(RequestIdSpan));
    axum::serve(listener, router).await?;

    Ok(())
}
