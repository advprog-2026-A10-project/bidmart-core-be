use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

use crate::modules::order::{create_router, infrastructure::create_app_state};

fn test_app() -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://postgres:postgres@localhost:5432/bidmart_test")
        .expect("valid lazy database url");
    let state = create_app_state(pool);
    create_router(state)
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    serde_json::from_slice(&bytes).expect("valid json response")
}

async fn first_order_id(app: &axum::Router) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/orders")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    body["data"][0]["id"]
        .as_str()
        .expect("first order id")
        .to_string()
}

async fn first_notification_id(app: &axum::Router) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/notifications")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    body["data"][0]["id"]
        .as_str()
        .expect("first notification id")
        .to_string()
}

#[tokio::test]
async fn list_orders_rejects_invalid_stage() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/orders?stage=not-valid")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn confirm_order_rejects_invalid_uuid() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/orders/not-a-uuid/confirm")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "actor_id": "buyer-vel" }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn confirm_order_returns_not_found_for_unknown_order() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/orders/{}/confirm", Uuid::new_v4()))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "actor_id": "buyer-vel" }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn dispute_requires_reason_phrase_barang_tidak_sesuai() {
    let app = test_app();
    let order_id = first_order_id(&app).await;
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/orders/{}/dispute/new", order_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "reporter_id": "buyer-vel",
                        "reason": "paket terlambat",
                        "details": "tes"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn update_shipping_rejects_invalid_status_value() {
    let app = test_app();
    let order_id = first_order_id(&app).await;
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/seller/orders/{}/shipping", order_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "status": "mystery-status",
                        "tracking": "TRK-123"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn mark_notification_read_twice_returns_conflict_on_second_call() {
    let app = test_app();
    let notification_id = first_notification_id(&app).await;
    let payload = json!({ "actorId": "buyer-vel" }).to_string();

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/notifications/{}/read", notification_id))
                .header("content-type", "application/json")
                .body(Body::from(payload.clone()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(first_response.status(), StatusCode::NO_CONTENT);

    let second_response = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/notifications/{}/read", notification_id))
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(second_response.status(), StatusCode::CONFLICT);
}
