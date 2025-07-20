use std::{future::IntoFuture, io};

use ferris_notify::{
    configuration::get_configurations,
    issue_delivery_worker::run_worker_until_stop,
    startup::build,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> io::Result<()> {
    let subscriber = get_subscriber("notify-server".into(), "debug".into(), io::stdout);
    init_subscriber(subscriber);

    let config = get_configurations().expect("Failed to read configuration.");

    let server = tokio::spawn(build(config.clone()).await.into_future());
    let worker = tokio::spawn(run_worker_until_stop(config));

    tokio::select! {
        _o = server=>{},
        _o = worker =>{}
    };

    Ok(())
}
