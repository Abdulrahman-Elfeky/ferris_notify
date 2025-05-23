use crate::helpers::{assert_is_redirect, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = spawn_app().await;

    let body = serde_json::json!({"username":"random-username","password":"random-password"});

    let res = app.post_login(&body).await;

    assert_is_redirect(&res, "/login");

    let flash_cookie = res.cookies().find(|c| c.name() == "_flash").unwrap();
    assert_eq!(flash_cookie.value().to_string(), "Authentication%20failed");

    let html = app.get_login_html().await;
    assert!(html.contains(r#"<p><i>Authentication failed</i></p>"#));
    // error should be ephemeral
    let html = app.get_login_html().await;
    assert!(!html.contains(r#"<p><i>Authentication failed</i></p>"#));
}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    let app = spawn_app().await;

    let body =
        serde_json::json!({"username":app.test_user.username,"password":app.test_user.password});

    let res = app.post_login(&body).await;

    assert_is_redirect(&res, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}!", app.test_user.username)));
}
