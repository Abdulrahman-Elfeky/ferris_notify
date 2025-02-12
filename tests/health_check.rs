mod common;

use common::setup;
use ferris_notify::configuration::get_configurations;
use reqwest::Client;
use sqlx::{query, Connection, PgConnection};

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

#[tokio::test]
async fn subscribe_return_200_for_a_valid_data() {
    let address = setup().await;
    let configuration = get_configurations().expect("Faild to read configuration.");
    let connection_string = configuration.database.get_connection_string();
    let mut connection = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connect to postgres.");

    let client = Client::new();
    let valid_data = "name=abdulrahman_elfeky&email=abdulrahmanelfeky7%40gmail.com";
    let res = client
        .post(format!("{}/subscriptions", address))
        .body(valid_data)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await
        .expect("Failed to execute request.");

    let saved = query!("SELECT email, name FROM subscriptions")
        .fetch_one(&mut connection)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(200, res.status().as_u16());
    assert_eq!(saved.name, "abdulrahman_elfeky")
}

#[tokio::test]
async fn subscribe_return_400_for_a_valid_data() {
    let client = Client::new();
    let address = setup().await;
    let invalid_data = [
        ("name=abdulrahman%20Elfeky", "email is missing"),
        ("email=abdulrahmanelfeky7%40gmail.com", "name is missing"),
        ("", "email and name is missing"),
    ];
    for (data, error_message) in invalid_data {
        let res = client
            .post(format!("{}/subscriptions", address))
            .body(data)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            422,
            res.status().as_u16(),
            "The API didn't fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}
