use axum::{
    extract::FromRef,
    routing::{get, post},
    serve::Serve,
    Router,
};

use secrecy::ExposeSecret;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::{
    configuration::Settings,
    email_client::EmailClient,
    routes::{health_check, subscribe},
    telemetry::RequestIdSpan,
};

#[derive(Clone)]
pub struct AppState {
    pub pg_pool: PgPool,
    pub email_client: EmailClient,
}

pub fn run(
    listener: TcpListener,
    pg_pool: PgPool,
    email_client: EmailClient,
) -> Serve<TcpListener, Router, Router> {
    let router = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(AppState {
            pg_pool,
            email_client,
        })
        .layer(TraceLayer::new_for_http().make_span_with(RequestIdSpan));
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

    let email_client =
        EmailClient::try_from(config.email_client).expect("Invalid email client settings.");

    run(listener, connection, email_client)
}

impl FromRef<AppState> for PgPool {
    fn from_ref(input: &AppState) -> Self {
        input.pg_pool.clone()
    }
}
impl FromRef<AppState> for EmailClient {
    fn from_ref(input: &AppState) -> Self {
        input.email_client.clone()
    }
}
