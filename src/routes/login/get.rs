use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
//use reqwest::header::CONTENT_TYPE;

pub async fn login_form(jar: CookieJar) -> impl IntoResponse {
    let error_message = match jar.get("_flash") {
        Some(c) => c.value(),
        None => "",
    }
    .to_string();

    let flash_message = Cookie::from("_flash");
    (
        StatusCode::OK,
        jar.remove(flash_message),
        Html(format!(
            r#"
    <!DOCTYPE html>
<html lang="en">

<head>
  <meta http-equiv="content-type" content="text/html; charset=utf-8">
  <title>Login</title>
</head>

<body>
<p><i>{error_message}</i></p>
  <form action="/login" method="post">
    <label>Username
      <input type="text" placeholder="Enter Username" name="username">
    </label>
    <label>Password
      <input type="password" placeholder="Enter Password" name="password">
    </label>
    <button type="submit">Login</button>
  </form>
</body>

</html>

                                  "#
        )),
    )
}
