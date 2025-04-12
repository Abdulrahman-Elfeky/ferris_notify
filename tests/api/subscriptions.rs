use sqlx::query;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_return_200_for_a_valid_data() {
    let app = spawn_app().await;

    let valid_data = "name=abdulrahman_elfeky&email=abdulrahmanelfeky7%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let res = app.post_subscriptions(valid_data).await;

    assert_eq!(200, res.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let app = spawn_app().await;

    let valid_data = "name=abdulrahman_elfeky&email=abdulrahmanelfeky7%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let _ = app.post_subscriptions(valid_data).await;

    let saved = query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.name, "abdulrahman_elfeky");
    assert_eq!(saved.email, "abdulrahmanelfeky7@gmail.com");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_return_422_for_an_missing_data() {
    let app = spawn_app().await;

    let invalid_data = [
        ("name=abdulrahman%20Elfeky", "email is missing"),
        ("email=abdulrahmanelfeky7%40gmail.com", "name is missing"),
        ("", "email and name is missing"),
    ];

    for (data, error_message) in invalid_data {
        let res = app.post_subscriptions(data).await;

        assert_eq!(
            422,
            res.status().as_u16(),
            "The API didn't fail with 422 Unprocessable Entity when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_return_400_invalid_data() {
    let app = spawn_app().await;

    let test_cases = [
        ("name=&email=abdulrahman%40gmail.com", "name is empty"),
        ("name=abdulrahman&email=", "email is empty"),
        ("name=&email=", "both name and email are empty"),
        ("name=abdulrahman&email=abdulrahman", "invalid email"),
    ];

    for (input, description) in test_cases {
        let res = app.post_subscriptions(input).await;

        assert_eq!(
            400,
            res.status().as_u16(),
            "The API didn't fail with 400 Bad Request when the payload was '{}'.",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_vaild_data() {
    let app = spawn_app().await;
    let body = "name=abdulrahman&email=abdulrahmanelfeky7%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let res = app.post_subscriptions(body).await;

    assert_eq!(res.status().as_u16(), 200);
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let app = spawn_app().await;
    let body = "name=abdulrahman&email=abdulrahmanelfeky7%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body).await;

    let _link = app.get_confirmation_link().await;
    //assert!(link.as_str().starts_with("http://127.0.0.1"));
}
