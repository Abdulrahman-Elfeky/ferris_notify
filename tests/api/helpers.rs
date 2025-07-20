use std::{env, io, net::SocketAddr, sync::LazyLock};

use argon2::{password_hash::SaltString, Algorithm, Argon2, Params, Version};
use ferris_notify::{
    configuration::{get_configurations, DatabaseSettings},
    email_client::{EmailClient, SendEmailRequest},
    issue_delivery_worker::ExecutionOutcome,
    startup::build,
    telemetry::{get_subscriber, init_subscriber},
};
use rand2::thread_rng;
use reqwest::redirect::Policy;
use secrecy::ExposeSecret;
use serde::Serialize;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

use ferris_notify::issue_delivery_worker::try_execute_task;
static TRACING: LazyLock<()> = LazyLock::new(|| {
    if env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber("test".into(), "debug".into(), io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber("test".into(), "debug".into(), io::sink);
        init_subscriber(subscriber);
    }
});
pub struct TestApp {
    pub address: SocketAddr,
    pub pool: PgPool,
    pub email_server: MockServer,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
    pub email_client: EmailClient,
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
    let serve = build(config.clone()).await;
    let address = serve.local_addr().unwrap();

    let fut = async || {
        serve.await.expect("Axum server stopped with error!!!");
    };
    tokio::spawn(fut());

    let api_client = reqwest::Client::builder()
        .redirect(Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    let email_client = config.email_client.client();

    let test_app = TestApp {
        address,
        pool,
        email_server,
        test_user: TestUser::generate(),
        api_client,
        email_client,
    };

    test_app.test_user.store(&test_app.pool).await;

    test_app
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
        self.api_client
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

    pub async fn publish_newsletter<Body: serde::Serialize>(
        &self,
        body: &Body,
    ) -> reqwest::Response {
        self.api_client
            .post(format!("http://{}/admin/newsletters", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_publish_newsletter(&self) -> reqwest::Response {
        self.api_client
            .get(format!("http://{}/admin/newsletters", self.address))
            .send()
            .await
            .expect("Failed to send request.")
    }

    pub async fn get_publish_newsletter_html(&self) -> String {
        self.get_publish_newsletter().await.text().await.unwrap()
    }

    pub async fn post_login<Body: serde::Serialize>(&self, body: &Body) -> reqwest::Response {
        self.api_client
            .post(format!("http://{}/login", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to send request.")
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(format!("http://{}/login", self.address))
            .send()
            .await
            .expect("Failed to send request.")
            .text()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(format!("http://{}/admin/dashboard", self.address))
            .send()
            .await
            .expect("Failed to send request.")
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }

    pub async fn get_change_password(&self) -> reqwest::Response {
        self.api_client
            .get(format!("http://{}/admin/password", self.address))
            .send()
            .await
            .expect("Failed to send request.")
    }

    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    pub async fn post_change_password<Body: Serialize>(&self, body: &Body) -> reqwest::Response {
        self.api_client
            .post(format!("http://{}/admin/password", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to send request.")
    }

    pub async fn post_logout(&self) -> reqwest::Response {
        self.api_client
            .post(format!("http://{}/admin/logout", self.address))
            .send()
            .await
            .expect("Failed to send request.")
    }

    pub async fn dispatch_all_pending_emails(&self) {
        loop {
            if let Ok(ExecutionOutcome::EmptyQueue) =
                try_execute_task(&self.pool, &self.email_client).await
            {
                break;
            }
        }
    }
}

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

use argon2::password_hash::PasswordHasher;
impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }
    pub async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut thread_rng());
        let hash_password = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

        sqlx::query!(
            "INSERT INTO users(user_id, username, password_hash) VALUES ($1,$2,$3)",
            self.user_id,
            self.username,
            hash_password
        )
        .execute(pool)
        .await
        .expect("Failed to insert test user.");
    }

    pub async fn login(&self, app: &TestApp) {
        app.post_login(&serde_json::json!({
            "username":self.username,
            "password":self.password,
        }))
        .await;
    }
}

pub fn assert_is_redirect(res: &reqwest::Response, path: &str) {
    assert_eq!(res.status().as_u16(), 303);
    assert_eq!(res.headers()["LOCATION"], path);
}
