use uuid::Uuid;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{spawn_app, TestApp};

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

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let body = r#" {"title":"some title", "html_content":"<h1>Welcome to our newsletters that's the first episode.</h1>"} "#;
    let res = app.publish_newsletter(body).await;
    assert_eq!(res.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let body = r#" {"title":"some title", "html_content":"<h1>Welcome to our newsletters that's the first episode.</h1>"} "#;
    let res = app.publish_newsletter(body).await;

    assert_eq!(res.status().as_u16(), 200);
}

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    let app = spawn_app().await;

    let request_body = r#" {"title":"some title", "html_content":"<h1>Welcome to our newsletters that's the first episode.</h1>"} "#;

    let res = reqwest::Client::new()
        .post(format!("http://{}/newsletter", app.address))
        .header("Content-Type", "application/json")
        .body(request_body)
        .send()
        .await
        .expect("Failed to send request.");

    assert_eq!(res.status().as_u16(), 401);
    assert_eq!(
        res.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    )
}

#[tokio::test]
async fn non_existing_users_are_rejected() {
    let app = spawn_app().await;

    let (username, password) = (Uuid::new_v4(), Uuid::new_v4());

    let request_body = r#" {"title":"some title", "html_content":"<h1>Welcome to our newsletters that's the first episode.</h1>"} "#;
    let res = reqwest::Client::new()
        .post(format!("http://{}/newsletter", app.address))
        .basic_auth(username, Some(password))
        .header("Content-Type", "application/json")
        .body(request_body)
        .send()
        .await
        .expect("Failed to send request.");

    assert_eq!(res.status().as_u16(), 401);
    assert_eq!(
        res.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    )
}
#[tokio::test]
async fn invalid_password_is_rejected() {
    let app = spawn_app().await;

    let (username, password) = (app.test_user.username, Uuid::new_v4());

    let request_body = r#" {"title":"some title", "html_content":"<h1>Welcome to our newsletters that's the first episode.</h1>"} "#;
    let res = reqwest::Client::new()
        .post(format!("http://{}/newsletter", app.address))
        .basic_auth(username, Some(password))
        .header("Content-Type", "application/json")
        .body(request_body)
        .send()
        .await
        .expect("Failed to send request.");

    assert_eq!(res.status().as_u16(), 401);
    assert_eq!(
        res.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    )
}
