//use std::time::Duration;

use ferris_notify::{configuration::get_configurations, startup::run};
use sqlx::PgPool;
use tokio::net::TcpListener;

pub async fn setup() -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to the address");
    let port = listener.local_addr().unwrap().port();
    let config = get_configurations().expect("Failed to read configuration.");
    let pool = PgPool::connect(&config.database.get_connection_string())
        .await
        .expect("Failed to connect to postgres.");
    let fut = run(listener, pool);
    tokio::spawn(fut);
    //tokio::time::sleep(Duration::from_secs(1)).await;
    format!("http://127.0.0.1:{}", port)
}
