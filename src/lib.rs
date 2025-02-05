use std::io;

use axum::{http::StatusCode, response::IntoResponse, routing::get, Router};
use tokio::net::TcpListener;

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

pub async fn run(listener: TcpListener) -> io::Result<()> {
    let router = Router::new().route("/health_check", get(health_check));

    //let listener = TcpListener::bind(address).await?;

    axum::serve(listener, router).await?;

    Ok(())
}
