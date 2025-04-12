use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_400() {
    let app = spawn_app().await;

    let res = reqwest::get(format!(
        "http://{}/subscriptions/confirm",
        app.address.to_string()
    ))
    .await
    .expect("Failed to send request.");

    assert_eq!(res.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_200_if_called() {
    let app = spawn_app().await;
    let body = "name=abdulrahman&email=abdulrahmanelfeky7%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new("200"))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let _ = app.post_subscriptions(body).await;

    let link = app.get_confirmation_link().await;

    let res = reqwest::get(link).await.expect("Failed to send request.");

    assert_eq!(res.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_the_subscriber() {
    let app = spawn_app().await;
    let body = "name=abdulrahman&email=abdulrahmanelfeky7%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new("200"))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let _ = app.post_subscriptions(body).await;

    let link = app.get_confirmation_link().await;

    reqwest::get(link)
        .await
        .expect("Failed to send request.")
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("SELECT name, email, status FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect("Faild to fetch saved subscription.");

    assert_eq!(saved.name, "abdulrahman");
    assert_eq!(saved.email, "abdulrahmanelfeky7@gmail.com");
    assert_eq!(saved.status, "confirmed");
}
