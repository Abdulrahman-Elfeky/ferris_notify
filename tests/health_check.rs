mod common;

use common::setup;
use reqwest::Client;

#[tokio::test]
async fn health_check_works() {
    let address = setup().await;

    let client = Client::new();
    let res = client
        .get(format!("{}/health_check", address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(res.status().is_success());
    assert_eq!(Some(0), res.content_length());
}
