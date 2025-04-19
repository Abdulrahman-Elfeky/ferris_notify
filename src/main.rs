use std::io;

use ferris_notify::{
    configuration::get_configurations,
    startup::build,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> io::Result<()> {
    let subscriber = get_subscriber("notify-server".into(), "debug".into(), io::stdout);
    init_subscriber(subscriber);

    let config = get_configurations().expect("Failed to read configuration.");

    let server = build(config).await;
    server.await?;
    Ok(())
}
