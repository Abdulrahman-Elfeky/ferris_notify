use reqwest::Client;

use crate::helpers::setup;

#[tokio::test]
async fn health_check_works() {
    let app = setup().await;

    let client = Client::new();
    let res = client
        .get(format!("http://{}/health_check", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(res.status().is_success());
    assert_eq!(Some(0), res.content_length());
}
