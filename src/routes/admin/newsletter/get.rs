use axum::response::{Html, IntoResponse, Response};
use axum_extra::extract::CookieJar;
use uuid::Uuid;

pub async fn publish_newsletter_form(jar: CookieJar) -> Result<impl IntoResponse, Response> {
    let idempotency_key = Uuid::new_v4();
    let msg_html = match jar.get("_flash") {
        Some(c) => c.value(),
        None => "",
    };

    Ok(Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Publish Newsletter Issue</title>
</head>
<body>
    <p><i>{msg_html}</i></p>
    <form action="/admin/newsletters" method="post">
        <label>Title:<br>
            <input
                type="text"
                placeholder="Enter the issue title"
                name="title"
            >
        </label>
        <br>
        <label>HTML content:<br>
            <textarea
                placeholder="Enter the content in HTML format"
                name="html_content"
                rows="20"
                cols="50"
            ></textarea>
        </label>
        <input hidden type="text" name="idempotency_key" value="{idempotency_key}">
        <button type="submit">Publish</button>
    </form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>"#,
    )))
}
