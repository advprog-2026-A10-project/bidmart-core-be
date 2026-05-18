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

async fn get_order(app: &axum::Router, order_id: &str) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/orders/{}", order_id))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await
}

async fn list_notifications_by_user(app: &axum::Router, user_id: &str) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/notifications?userId={}", user_id))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await
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

#[tokio::test]
async fn confirm_order_updates_order_state_to_completed_and_delivered() {
    let app = test_app();
    let order_id = first_order_id(&app).await;

    let confirm_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/orders/{}/confirm", order_id))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "actor_id": "buyer-vel" }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(confirm_response.status(), StatusCode::NO_CONTENT);

    let order = get_order(&app, &order_id).await;
    assert_eq!(order["status"], "Delivered");
    assert_eq!(
        order["stage"].as_str().expect("stage").to_lowercase(),
        "completed"
    );
}

#[tokio::test]
async fn create_dispute_updates_order_state_and_tags() {
    let app = test_app();
    let order_id = first_order_id(&app).await;
    let reason = "Barang tidak sesuai dengan foto";
    let details = "Warna berbeda dan ada goresan";

    let dispute_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/orders/{}/dispute/new", order_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "reporter_id": "buyer-vel",
                        "reason": reason,
                        "details": details
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(dispute_response.status(), StatusCode::NO_CONTENT);

    let order = get_order(&app, &order_id).await;
    assert_eq!(order["status"], "Dispute Alert");
    assert_eq!(
        order["stage"].as_str().expect("stage").to_lowercase(),
        "processing"
    );

    let tags = order["tags"].as_array().expect("tags");
    assert!(tags.iter().any(|tag| {
        tag.as_str()
            .map(|value| value.contains("Dispute reason:"))
            .unwrap_or(false)
    }));
    assert!(tags.iter().any(|tag| {
        tag.as_str()
            .map(|value| value.contains("Dispute details:"))
            .unwrap_or(false)
    }));
}

#[tokio::test]
async fn update_shipping_stores_tracking_and_transitions_order() {
    let app = test_app();
    let order_id = first_order_id(&app).await;

    let shipping_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/seller/orders/{}/shipping", order_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "status": "in_transit",
                        "tracking": "TRK-9999"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(shipping_response.status(), StatusCode::NO_CONTENT);

    let order = get_order(&app, &order_id).await;
    assert_eq!(order["status"], "In Transit");
    assert_eq!(
        order["stage"].as_str().expect("stage").to_lowercase(),
        "processing"
    );
    let tags = order["tags"].as_array().expect("tags");
    assert!(tags.iter().any(|tag| {
        tag.as_str()
            .map(|value| value == "Tracking: TRK-9999")
            .unwrap_or(false)
    }));
}

#[tokio::test]
async fn publish_event_adds_notification_for_target_user() {
    let app = test_app();
    let before = list_notifications_by_user(&app, "buyer-vel").await;
    let before_count = before["data"].as_array().expect("data").len();

    let publish_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/events/notifications")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "orderId": null,
                        "type": "OrderUpdate",
                        "title": "Order updated for buyer-vel",
                        "body": "Your order status moved to processing.",
                        "channel": "inbox",
                        "metadata": {
                            "userId": "buyer-vel"
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(publish_response.status(), StatusCode::NO_CONTENT);

    let after = list_notifications_by_user(&app, "buyer-vel").await;
    let after_data = after["data"].as_array().expect("data");
    assert_eq!(after_data.len(), before_count + 1);
    assert!(after_data.iter().any(|notification| {
        notification["title"]
            .as_str()
            .map(|title| title == "Order updated for buyer-vel")
            .unwrap_or(false)
    }));
}

#[tokio::test]
async fn mark_notification_read_parallel_requests_are_consistent() {
    let app = test_app();
    let notification_id = first_notification_id(&app).await;
    let payload = json!({ "actorId": "buyer-vel" }).to_string();

    let request_a = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/notifications/{}/read", notification_id))
        .header("content-type", "application/json")
        .body(Body::from(payload.clone()))
        .expect("request");

    let request_b = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/notifications/{}/read", notification_id))
        .header("content-type", "application/json")
        .body(Body::from(payload))
        .expect("request");

    let app_a = app.clone();
    let app_b = app.clone();
    let (response_a, response_b) = tokio::join!(app_a.oneshot(request_a), app_b.oneshot(request_b));
    let status_a = response_a.expect("response A").status();
    let status_b = response_b.expect("response B").status();

    let mut statuses = vec![status_a, status_b];
    statuses.sort();
    assert_eq!(statuses, vec![StatusCode::NO_CONTENT, StatusCode::CONFLICT]);
}
