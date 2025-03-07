use reqwest::Client;
use sqlx::query;

use crate::helpers::setup;

#[tokio::test]
async fn subscribe_return_200_for_a_valid_data() {
    let app = setup().await;
    let client = Client::new();
    let valid_data = "name=abdulrahman_elfeky&email=abdulrahmanelfeky7%40gmail.com";
    let res = client
        .post(format!("http://{}/subscriptions", app.address))
        .body(valid_data)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await
        .expect("Failed to execute request.");

    let saved = query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(200, res.status().as_u16());
    assert_eq!(saved.name, "abdulrahman_elfeky")
}

#[tokio::test]
async fn subscribe_return_422_for_an_invalid_data() {
    let client = Client::new();
    let app = setup().await;
    let invalid_data = [
        ("name=abdulrahman%20Elfeky", "email is missing"),
        ("email=abdulrahmanelfeky7%40gmail.com", "name is missing"),
        ("", "email and name is missing"),
    ];
    for (data, error_message) in invalid_data {
        let res = client
            .post(format!("http://{}/subscriptions", app.address))
            .body(data)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            422,
            res.status().as_u16(),
            "The API didn't fail with 422 Unprocessable Entity when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_return_400_empty_data() {
    let client = Client::new();
    let app = setup().await;
    let test_cases = [
        ("name=&email=abdulrahman%40gmail.com", "name is empty"),
        ("name=abdulrahman&email=", "email is empty"),
        ("name=&email=", "both name and email are empty"),
    ];

    for (input, description) in test_cases {
        let res = client
            .post(format!("http://{}/subscriptions", app.address))
            .body(input)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            res.status().as_u16(),
            "The API didn't fail with 400 Bad Request when the payload was '{}'.",
            description
        );
    }
}
