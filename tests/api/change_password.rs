use uuid::Uuid;

use crate::helpers::{assert_is_redirect, spawn_app};

#[tokio::test]
async fn u_must_be_logged_in_to_see_change_password_form() {
    let app = spawn_app().await;

    let res = app.get_change_password().await;

    assert_is_redirect(&res, "/login");
}

#[tokio::test]
async fn u_must_be_logged_in_to_change_your_password() {
    let app = spawn_app().await;

    let new_password = Uuid::new_v4().to_string();

    let body = serde_json::json!({
        "current_password":Uuid::new_v4().to_string(),
        "new_password":&new_password,
        "new_password_check":&new_password});

    let res = app.post_change_password(&body).await;

    assert_is_redirect(&res, "/login");
}

#[tokio::test]
async fn new_passwords_must_match() {
    let app = spawn_app().await;

    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    app.post_login(&serde_json::json!({
        "username":app.test_user.username,
        "password": app.test_user.password,
    }))
    .await;

    let res = app
        .post_change_password(&serde_json::json!({
            "current_password":app.test_user.password,
            "new_password":new_password,
            "new_password_check":another_new_password
        }))
        .await;
    assert_is_redirect(&res, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - the field values must match.</i></p>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;

    app.post_login(&serde_json::json!({
        "username":app.test_user.username,
        "password": app.test_user.password,
    }))
    .await;

    let wrong_passowrd = Uuid::new_v4().to_string();
    let new_passowrd = Uuid::new_v4().to_string();

    let res = app
        .post_change_password(&serde_json::json!({
            "current_password":wrong_passowrd,
            "new_password":new_passowrd,
            "new_password_check":new_passowrd,
        }))
        .await;

    assert_is_redirect(&res, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The current password is incorrect.</i></p>"))
}

#[tokio::test]
async fn changing_password_works() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // login
    let res = app
        .post_login(&serde_json::json!({
            "username":app.test_user.username,
            "password":app.test_user.password,
        }))
        .await;

    assert_is_redirect(&res, "/admin/dashboard");

    // change password
    let res = app
        .post_change_password(&serde_json::json!({
            "current_password":app.test_user.password,
            "new_password":new_password,
            "new_password_check":new_password,
        }))
        .await;

    assert_is_redirect(&res, "/admin/password");

    // follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(&"<p><i>Your password has been changed.</i></p>"));

    // logout
    let res = app.post_logout().await;

    assert_is_redirect(&res, "/login");

    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>You have successfully logged out.</i></p>"#,));

    // login using the new password
    let res = app
        .post_login(&serde_json::json!({
            "username":app.test_user.username,
            "password":new_password,
        }))
        .await;
    assert_is_redirect(&res, "/admin/dashboard");
}
