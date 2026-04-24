use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{TimeZone, Utc};
use http_body_util::BodyExt;
use rastraq::{app::build_router, db::Database};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn item_to_daily_edition_to_feedback_flow() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    db.migrate().await.unwrap();
    let app = build_router(db);

    let item_body = json!({
        "url": "https://example.com/rust-security",
        "title": "Rust security advisory",
        "source_type": "security_advisory",
        "published_at": Utc.with_ymd_and_hms(2026, 4, 23, 3, 0, 0).unwrap(),
        "raw_content": "Rust crate maintainers published a security fix and release notes."
    });
    let response = app
        .clone()
        .oneshot(json_request("POST", "/api/items", item_body))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = response_json(response).await;
    let item_id = created["id"].as_i64().unwrap();

    let response = app
        .clone()
        .oneshot(json_request("POST", &format!("/api/items/{item_id}/process"), json!({})))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/editions/generate",
            json!({"now": "2026-04-24T01:00:00Z"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .clone()
        .oneshot(json_request("GET", "/api/editions/today?now=2026-04-24T01:00:00Z", json!({})))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let today = response_json(response).await;
    assert_eq!(today["items"].as_array().unwrap().len(), 1);
    assert_eq!(today["items"][0]["id"].as_i64().unwrap(), item_id);

    let response = app
        .oneshot(json_request(
            "POST",
            "/api/feedback",
            json!({"item_id": item_id, "event_type": "interested", "payload": {"surface": "card"}}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

fn json_request(method: &str, path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}
