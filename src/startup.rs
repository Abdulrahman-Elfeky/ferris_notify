use std::io;

use axum::{
    routing::{get, post},
    Router,
};

use sqlx::PgPool;
use tokio::net::TcpListener;

use crate::routes::{health_check, subscribe};

#[derive(Clone, Debug)]
pub struct AppState {
    pub pg_pool: PgPool,
}
pub async fn run(listener: TcpListener, pg_pool: PgPool) -> io::Result<()> {
    let router = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(AppState { pg_pool });

    //let listener = TcpListener::bind(address).await?;

    axum::serve(listener, router).await?;

    Ok(())
}
