use std::io;

use ferris_notify::{configuration::get_configurations, startup::run};
use sqlx::PgPool;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> io::Result<()> {
    let config = get_configurations().expect("Failed to read configuration.");
    let address = format!("127.0.0.1:{}", config.application_port);
    let listener = TcpListener::bind(address)
        .await
        .expect("Failed to bind to address.");
    let connection = PgPool::connect(&config.database.get_connection_string())
        .await
        .expect("Failed to connect to postgres.");
    run(listener, connection).await?;
    Ok(())
}
