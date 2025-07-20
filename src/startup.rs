use std::sync::Arc;

use axum::{
    extract::FromRef,
    middleware,
    routing::{get, post},
    serve::Serve,
    Router,
};

use reqwest::Method;
use secrecy::SecretString;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultOnFailure, TraceLayer},
};
use tower_sessions::{
    cookie::{time::Duration, Key},
    Expiry, SessionManagerLayer,
};
use tower_sessions_redis_store::{
    fred::prelude::{ClientLike, Config, Pool},
    RedisStore,
};

use crate::{
    authentication::reject_anonymous_users,
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{
        admin_dashboard, change_password, change_password_form, confirm, health_check, home,
        log_out, login, login_form, publish_newsletter, publish_newsletter_form, subscribe,
    },
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
    _hmac_secret: SecretString,
) -> Serve<TcpListener, Router, Router> {
    let base_url = Arc::new(BaseUrl(base_url));

    let pool = Pool::new(Config::default(), None, None, None, 6).unwrap();
    let _ = pool.connect();
    let session_store = RedisStore::new(pool);
    let key = Key::generate();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_signed(key)
        .with_expiry(Expiry::OnInactivity(Duration::seconds(120)));

    let admin_router = Router::new()
        .route("/dashboard", get(admin_dashboard))
        .route("/newsletters", post(publish_newsletter))
        .route("/newsletters", get(publish_newsletter_form))
        .route("/password", get(change_password_form))
        .route("/password", post(change_password))
        .route("/logout", post(log_out))
        .layer(middleware::from_fn(reject_anonymous_users));
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any);
    let router = Router::new()
        .route("/", get(home))
        .route("/login", get(login_form))
        .route("/login", post(login))
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .nest("/admin", admin_router)
        .with_state(AppState {
            pg_pool,
            email_client,
            base_url,
        })
        .layer(
            ServiceBuilder::new()
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(RequestIdSpan)
                        .on_failure(DefaultOnFailure::new()),
                )
                .layer(cors)
                .layer(session_layer),
        );
    axum::serve(listener, router)
}

pub async fn build(config: Settings) -> Serve<TcpListener, Router, Router> {
    let address = format!("{}:{}", config.application.host, config.application.port);
    let listener = TcpListener::bind(address)
        .await
        .expect("Failed to bind to address.");

    let connection = get_connection_pool(&config.database);

    let email_client =
        EmailClient::try_from(config.email_client).expect("Invalid email client settings.");

    run(
        listener,
        connection,
        email_client,
        config.application.base_url,
        config.application.hmac_secret,
    )
}

pub fn get_connection_pool(config: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(config.connect_options())
}
