use axum::{
    http::{header::CONTENT_TYPE, StatusCode},
    response::IntoResponse,
};

pub async fn home() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("home.html"),
    )
}
