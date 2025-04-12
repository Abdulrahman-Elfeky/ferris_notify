use std::sync::Arc;

use axum::{
    extract::FromRef,
    routing::{get, post},
    serve::Serve,
    Router,
};

use secrecy::ExposeSecret;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultOnFailure, TraceLayer};

use crate::{
    configuration::Settings,
    email_client::EmailClient,
    routes::{confirm, health_check, publish_newsletter, subscribe},
    telemetry::RequestIdSpan,
};

#[derive(Clone, FromRef)]
pub struct AppState {
    pub pg_pool: PgPool,
    pub email_client: EmailClient,
    pub base_url: Arc<BaseUrl>,
}

pub struct BaseUrl(pub String);
pub fn run(
    listener: TcpListener,
    pg_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Serve<TcpListener, Router, Router> {
    let base_url = Arc::new(BaseUrl(base_url));
    let router = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .route("/newsletter", post(publish_newsletter))
        .with_state(AppState {
            pg_pool,
            email_client,
            base_url,
        })
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(RequestIdSpan)
                .on_failure(DefaultOnFailure::new()),
        );
    axum::serve(listener, router)
}

pub async fn build(config: Settings) -> Serve<TcpListener, Router, Router> {
    let address = format!("{}:{}", config.application.host, config.application.port);
    let listener = TcpListener::bind(address)
        .await
        .expect("Failed to bind to address.");

    let connection = PgPool::connect_lazy(&config.database.get_connection_string().expose_secret())
        //.await
        .expect("Failed to connect to postgres.");

    dbg!(&config.email_client.base_url);
    let email_client =
        EmailClient::try_from(config.email_client).expect("Invalid email client settings.");

    run(
        listener,
        connection,
        email_client,
        config.application.base_url,
    )
}
