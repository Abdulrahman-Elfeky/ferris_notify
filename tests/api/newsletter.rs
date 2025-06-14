use std::time::Duration;

use uuid::Uuid;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{assert_is_redirect, spawn_app, TestApp};

async fn create_unconfirmed_subscriber(app: &TestApp) -> reqwest::Url {
    let body = "name=abdulrahman_elfeky&email=abdulrahmanelfeky7%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body)
        .await
        .error_for_status()
        .unwrap();

    app.get_confirmation_link().await
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let link = create_unconfirmed_subscriber(app).await;
    reqwest::get(link)
        .await
        .expect("Failed to send request to confirm the subscriber.")
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;

    create_unconfirmed_subscriber(&app).await;

    app.test_user.login(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let body = serde_json::json!({
        "title":"some title",
        "html_content":"<h1>Welcome to our newsletters that's the first episode.</h1>",
        "idempotency_key": Uuid::new_v4().to_string(),
    });
    let res = app.publish_newsletter(&body).await;

    assert_is_redirect(&res, "/admin/newsletters");

    let html = app.get_publish_newsletter_html().await;
    assert!(html.contains("<p><i>The newsletter issue has been published!</i></p>"));
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;

    app.test_user.login(&app).await;

    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let body = serde_json::json!({
        "title":"some title",
        "html_content":"<h1>Welcome to our newsletters that's the first episode.</h1>",
        "idempotency_key": Uuid::new_v4().to_string(),
    });

    let res = app.publish_newsletter(&body).await;

    assert_is_redirect(&res, "/admin/newsletters");

    let html = app.get_publish_newsletter_html().await;
    assert!(html.contains("<p><i>The newsletter issue has been published!</i></p>"));
}

#[tokio::test]
async fn u_must_be_logged_in_to_publish_newsletter() {
    let app = spawn_app().await;

    let body = serde_json::json!({
        "title":"some title",
        "html_content":"<h1>Welcome to our newsletters that's the first episode.</h1>",
        "idempotency_key": Uuid::new_v4().to_string(),
    });

    let res = app.publish_newsletter(&body).await;

    assert_is_redirect(&res, "/login");
}

#[tokio::test]
async fn u_must_be_logged_in_to_see_the_newsletter_form() {
    let app = spawn_app().await;

    let res = app.get_publish_newsletter().await;

    assert_is_redirect(&res, "/login");
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;

    app.test_user.login(&app).await;

    let body = serde_json::json!({
        "title":"some title",
        "html_content":"<h1>Welcome to our newsletters that's the first episode.</h1>",
        "idempotency_key":Uuid::new_v4().to_string()
    });

    // just one email is sent
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let res = app.publish_newsletter(&body).await;

    assert_is_redirect(&res, "/admin/newsletters");

    let html = app.get_publish_newsletter_html().await;
    assert!(html.contains("<p><i>The newsletter issue has been published!</i></p>"));

    let res = app.publish_newsletter(&body).await;
    assert_is_redirect(&res, "/admin/newsletters");

    let html = app.get_publish_newsletter_html().await;
    assert!(html.contains("<p><i>The newsletter issue has been published!</i></p>"));
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;

    app.test_user.login(&app).await;

    let body = serde_json::json!({
        "title":"some title",
        "html_content":"<h1>Welcome to our newsletters that's the first episode.</h1>",
        "idempotency_key":Uuid::new_v4().to_string()
    });

    Mock::given(method("POST"))
        .and(path("/email"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let res1 = app.publish_newsletter(&body);
    let res2 = app.publish_newsletter(&body);

    let (res1, res2) = tokio::join!(res1, res2);

    assert_eq!(res1.status(), res2.status());
    assert_eq!(res1.text().await.unwrap(), res2.text().await.unwrap());
}
