use std::{env, io, net::SocketAddr, sync::LazyLock};

use ferris_notify::{
    configuration::{get_configurations, DatabaseSettings},
    email_client::SendEmailRequest,
    startup::build,
    telemetry::{get_subscriber, init_subscriber},
};
use reqwest::Client;
use secrecy::ExposeSecret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

static TRACING: LazyLock<()> = LazyLock::new(|| {
    if env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber("test".into(), "info".into(), io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber("test".into(), "info".into(), io::sink);
        init_subscriber(subscriber);
    }
});
pub struct TestApp {
    pub address: SocketAddr,
    pub pool: PgPool,
    pub email_server: MockServer,
}

pub async fn spawn_app() -> TestApp {
    LazyLock::force(&TRACING);

    let email_server = MockServer::start().await;

    let config = {
        let mut c = get_configurations().expect("Failed to read configuration.");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };
    let pool = configure_database(&config.database).await;
    let serve = build(config).await;
    let address = serve.local_addr().unwrap();

    let fut = async || {
        serve.await.expect("Axum server stopped with error!!!");
    };
    tokio::spawn(fut());

    TestApp {
        address,
        pool,
        email_server,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection =
        PgConnection::connect(&config.get_connection_string_without_db().expose_secret())
            .await
            .expect("Failed to connect to postgres.");

    connection
        .execute(format!(r#"CREATE DATABASE "{}" ;"#, config.database_name).as_str())
        .await
        .expect("Failed to create the database.");
    let pool = PgPool::connect(&config.get_connection_string().expose_secret())
        .await
        .expect("Failed to connect to postgres");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate the database.");
    pool
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: &'static str) -> reqwest::Response {
        let client = Client::new();
        client
            .post(format!("http://{}/subscriptions", self.address))
            .body(body)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_confirmation_link(&self) -> reqwest::Url {
        let email_body = self.email_server.received_requests().await.unwrap()[0]
            .body_json::<SendEmailRequest>()
            .unwrap()
            .html_content;

        let links = linkify::LinkFinder::new()
            .links(&email_body)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect::<Vec<_>>();

        assert_eq!(links.len(), 1);
        let mut link = reqwest::Url::parse(links[0].as_str()).unwrap();

        link.set_port(Some(self.address.port())).unwrap();
        link
    }

    pub async fn publish_newsletter(&self, body: &'static str) -> reqwest::Response {
        let client = Client::new();
        client
            .post(format!("http://{}/newsletter", self.address))
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await
            .expect("Failed to execute request.")
    }
}
