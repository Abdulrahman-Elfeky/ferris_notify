use std::io;

use ferris_notify::{
    configuration::get_configurations,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};
use secrecy::ExposeSecret;
use sqlx::PgPool;
use tokio::net::TcpListener;
//use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> io::Result<()> {
    let subscriber = get_subscriber("notify-server".into(), "debug".into(), io::stdout);
    init_subscriber(subscriber);
    //tracing_subscriber::fmt()
    //    .with_env_filter(EnvFilter::from_default_env())
    //    .init();
    let config = get_configurations().expect("Failed to read configuration.");
    let address = format!("127.0.0.1:{}", config.application_port);
    let listener = TcpListener::bind(address)
        .await
        .expect("Failed to bind to address.");
    let connection = PgPool::connect(&config.database.get_connection_string().expose_secret())
        .await
        .expect("Failed to connect to postgres.");
    run(listener, connection).await?;
    Ok(())
}
