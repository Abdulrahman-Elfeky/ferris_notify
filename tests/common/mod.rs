//use std::time::Duration;

use std::{env, io, sync::LazyLock};

use ferris_notify::{
    configuration::{get_configurations, DatabaseSettings},
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};
use secrecy::ExposeSecret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tokio::net::TcpListener;
use uuid::Uuid;

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
    pub address: String,
    pub pool: PgPool,
}

pub async fn setup() -> TestApp {
    LazyLock::force(&TRACING);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to the address");
    let port = listener.local_addr().unwrap().port();
    let mut config = get_configurations().expect("Failed to read configuration.");
    config.database.database_name = Uuid::new_v4().to_string();
    //dbg!(&config.database);
    let pool = configure_database(&config.database).await;
    let fut = run(listener, pool.clone());
    tokio::spawn(fut);
    //tokio::time::sleep(Duration::from_secs(1)).await;
    let address = format!("http://127.0.0.1:{}", port);
    TestApp { address, pool }
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
