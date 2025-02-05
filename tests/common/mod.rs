use std::time::Duration;

use ferris_notify::run;
use tokio::net::TcpListener;

pub async fn setup() -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to the address");
    let port = listener.local_addr().unwrap().port();
    let fut = run(listener);
    tokio::spawn(fut);
    tokio::time::sleep(Duration::from_secs(1)).await;
    format!("http://127.0.0.1:{}", port)
}
