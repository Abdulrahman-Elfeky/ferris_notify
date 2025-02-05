use std::io;

use ferris_notify::run;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to address.");
    run(listener).await?;
    Ok(())
}
