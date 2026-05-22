use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::Instant;

use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tower::ServiceExt;
use uuid::Uuid;

use crate::modules::order::create_router;
use crate::modules::order::infrastructure::repositories::{
    db_notification_repository::DbNotificationRepository, db_order_repository::DbOrderRepository,
};
use crate::modules::order::infrastructure::{
    AppState, NotificationRepositoryHandle, OrderRepositoryHandle,
};

use super::support::create_app_state;

const TEST_BUYER_ONE_ID: &str = "11111111-1111-1111-1111-111111111111";
const TEST_SELLER_ONE_ID: &str = "22222222-2222-2222-2222-222222222222";
const TEST_BUYER_TWO_ID: &str = "33333333-3333-3333-3333-333333333333";
const DEFAULT_IN_MEMORY_PROFILE_ITERATIONS: usize = 200;
const DEFAULT_IN_MEMORY_PROFILE_WARMUP: usize = 20;
const DEFAULT_IN_MEMORY_PROFILE_APDEX_MS: f64 = 10.0;
const PROFILE_BUYER_ID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
const PROFILE_SELLER_ID: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
const PROFILE_CATEGORY_SLUG: &str = "performance-order-notification";

fn test_app() -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://postgres:postgres@localhost:5432/bidmart_test")
        .expect("valid lazy database url");
    let state = create_app_state(pool);
    create_router(state)
}

fn test_app_with_auth(auth_base_url: String) -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://postgres:postgres@localhost:5432/bidmart_test")
        .expect("valid lazy database url");
    let mut state = create_app_state(pool);
    state.auth_base_url = auth_base_url;
    create_router(state)
}

fn test_app_with_internal_secret(secret: &str) -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://postgres:postgres@localhost:5432/bidmart_test")
        .expect("valid lazy database url");
    let mut state = create_app_state(pool);
    state.internal_secret = Some(Arc::<str>::from(secret.to_string()));
    create_router(state)
}

#[derive(Clone)]
struct MockAuthState {
    forced_status: Option<StatusCode>,
    token_to_user_id: Arc<HashMap<String, Uuid>>,
}

async fn mock_auth_validate(
    State(state): State<MockAuthState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(status) = state.forced_status {
        return status.into_response();
    }

    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let Some(token_text) = token else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let Some(user_id) = state.token_to_user_id.get(token_text).copied() else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    (
        StatusCode::OK,
        Json(json!({
            "userId": user_id,
            "name": "Mock User",
            "email": "mock@bidmart.test",
            "emailVerified": true,
            "mfaSatisfied": true,
            "sessionExpiry": "2099-01-01T00:00:00Z"
        })),
    )
        .into_response()
}

async fn spawn_mock_auth_server(
    token_to_user_id: HashMap<String, Uuid>,
    forced_status: Option<StatusCode>,
) -> (String, JoinHandle<()>) {
    let app = Router::new()
        .route("/auth/validate", post(mock_auth_validate))
        .with_state(MockAuthState {
            forced_status,
            token_to_user_id: Arc::new(token_to_user_id),
        });

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock auth listener");
    let address = listener.local_addr().expect("mock auth listener address");
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    (format!("http://{}", address), handle)
}

async fn first_order_id_with_bearer(app: &axum::Router, token: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/orders")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    body["data"][0]["id"]
        .as_str()
        .expect("first order id with bearer")
        .to_string()
}

async fn first_notification_id_with_bearer(app: &axum::Router, token: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/notifications")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    body["data"][0]["id"]
        .as_str()
        .expect("first notification id with bearer")
        .to_string()
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
async fn list_orders_requires_auth_when_integration_enabled() {
    let (auth_base_url, auth_handle) = spawn_mock_auth_server(HashMap::new(), None).await;
    let app = test_app_with_auth(auth_base_url);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/orders")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    auth_handle.abort();
}

#[tokio::test]
async fn list_orders_allows_authenticated_buyer_scope() {
    let mut token_to_user_id = HashMap::new();
    token_to_user_id.insert(
        "buyer-one-token".to_string(),
        Uuid::parse_str(TEST_BUYER_ONE_ID).expect("buyer one uuid"),
    );

    let (auth_base_url, auth_handle) = spawn_mock_auth_server(token_to_user_id, None).await;
    let app = test_app_with_auth(auth_base_url);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/orders")
                .header("authorization", "Bearer buyer-one-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let orders = body["data"].as_array().expect("orders array");
    assert_eq!(orders.len(), 1);
    assert_eq!(
        orders[0]["buyerId"].as_str().expect("buyer id"),
        TEST_BUYER_ONE_ID
    );
    auth_handle.abort();
}

#[tokio::test]
async fn get_order_returns_forbidden_for_non_owner_when_auth_enabled() {
    let mut token_to_user_id = HashMap::new();
    token_to_user_id.insert(
        "buyer-one-token".to_string(),
        Uuid::parse_str(TEST_BUYER_ONE_ID).expect("buyer one uuid"),
    );
    token_to_user_id.insert(
        "buyer-two-token".to_string(),
        Uuid::parse_str(TEST_BUYER_TWO_ID).expect("buyer two uuid"),
    );

    let (auth_base_url, auth_handle) = spawn_mock_auth_server(token_to_user_id, None).await;
    let app = test_app_with_auth(auth_base_url);
    let order_id = first_order_id_with_bearer(&app, "buyer-one-token").await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/orders/{}", order_id))
                .header("authorization", "Bearer buyer-two-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    auth_handle.abort();
}

#[tokio::test]
async fn seller_shipping_update_returns_forbidden_for_buyer_actor() {
    let mut token_to_user_id = HashMap::new();
    token_to_user_id.insert(
        "buyer-one-token".to_string(),
        Uuid::parse_str(TEST_BUYER_ONE_ID).expect("buyer one uuid"),
    );

    let (auth_base_url, auth_handle) = spawn_mock_auth_server(token_to_user_id, None).await;
    let app = test_app_with_auth(auth_base_url);
    let order_id = first_order_id_with_bearer(&app, "buyer-one-token").await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/seller/orders/{}/shipping", order_id))
                .header("authorization", "Bearer buyer-one-token")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "status": "in_transit",
                        "tracking": "TRK-AUTH-1"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    auth_handle.abort();
}

#[tokio::test]
async fn seller_shipping_update_allows_authenticated_seller() {
    let mut token_to_user_id = HashMap::new();
    token_to_user_id.insert(
        "buyer-one-token".to_string(),
        Uuid::parse_str(TEST_BUYER_ONE_ID).expect("buyer one uuid"),
    );
    token_to_user_id.insert(
        "seller-one-token".to_string(),
        Uuid::parse_str(TEST_SELLER_ONE_ID).expect("seller one uuid"),
    );

    let (auth_base_url, auth_handle) = spawn_mock_auth_server(token_to_user_id, None).await;
    let app = test_app_with_auth(auth_base_url);
    let order_id = first_order_id_with_bearer(&app, "buyer-one-token").await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/seller/orders/{}/shipping", order_id))
                .header("authorization", "Bearer seller-one-token")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "status": "in_transit",
                        "tracking": "TRK-AUTH-2"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    auth_handle.abort();
}

#[tokio::test]
async fn list_orders_returns_bad_gateway_when_auth_validate_upstream_fails() {
    let (auth_base_url, auth_handle) =
        spawn_mock_auth_server(HashMap::new(), Some(StatusCode::INTERNAL_SERVER_ERROR)).await;
    let app = test_app_with_auth(auth_base_url);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/orders")
                .header("authorization", "Bearer any-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    auth_handle.abort();
}

#[tokio::test]
async fn list_notifications_requires_auth_when_integration_enabled() {
    let (auth_base_url, auth_handle) = spawn_mock_auth_server(HashMap::new(), None).await;
    let app = test_app_with_auth(auth_base_url);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/notifications")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    auth_handle.abort();
}

#[tokio::test]
async fn list_notifications_is_scoped_to_authenticated_user() {
    let mut token_to_user_id = HashMap::new();
    token_to_user_id.insert(
        "buyer-one-token".to_string(),
        Uuid::parse_str(TEST_BUYER_ONE_ID).expect("buyer one uuid"),
    );

    let (auth_base_url, auth_handle) = spawn_mock_auth_server(token_to_user_id, None).await;
    let app = test_app_with_auth(auth_base_url);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/notifications")
                .header("authorization", "Bearer buyer-one-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let notifications = body["data"].as_array().expect("notifications");
    assert_eq!(notifications.len(), 1);
    auth_handle.abort();
}

#[tokio::test]
async fn get_notification_returns_forbidden_for_non_owner_when_auth_enabled() {
    let mut token_to_user_id = HashMap::new();
    token_to_user_id.insert(
        "buyer-one-token".to_string(),
        Uuid::parse_str(TEST_BUYER_ONE_ID).expect("buyer one uuid"),
    );
    token_to_user_id.insert(
        "buyer-two-token".to_string(),
        Uuid::parse_str(TEST_BUYER_TWO_ID).expect("buyer two uuid"),
    );

    let (auth_base_url, auth_handle) = spawn_mock_auth_server(token_to_user_id, None).await;
    let app = test_app_with_auth(auth_base_url);
    let notification_id = first_notification_id_with_bearer(&app, "buyer-one-token").await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/notifications/{}", notification_id))
                .header("authorization", "Bearer buyer-two-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    auth_handle.abort();
}

#[tokio::test]
async fn get_notification_rejects_invalid_uuid() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/notifications/not-a-uuid")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn mark_notification_read_rejects_invalid_uuid() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/notifications/not-a-uuid/read")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "actorId": TEST_BUYER_ONE_ID }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn mark_notification_read_returns_forbidden_for_non_owner_when_auth_enabled() {
    let mut token_to_user_id = HashMap::new();
    token_to_user_id.insert(
        "buyer-one-token".to_string(),
        Uuid::parse_str(TEST_BUYER_ONE_ID).expect("buyer one uuid"),
    );
    token_to_user_id.insert(
        "buyer-two-token".to_string(),
        Uuid::parse_str(TEST_BUYER_TWO_ID).expect("buyer two uuid"),
    );

    let (auth_base_url, auth_handle) = spawn_mock_auth_server(token_to_user_id, None).await;
    let app = test_app_with_auth(auth_base_url);
    let notification_id = first_notification_id_with_bearer(&app, "buyer-one-token").await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/notifications/{}/read", notification_id))
                .header("authorization", "Bearer buyer-two-token")
                .header("content-type", "application/json")
                .body(Body::from(json!({}).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    auth_handle.abort();
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
async fn confirm_order_returns_forbidden_for_non_buyer_when_auth_enabled() {
    let mut token_to_user_id = HashMap::new();
    token_to_user_id.insert(
        "buyer-one-token".to_string(),
        Uuid::parse_str(TEST_BUYER_ONE_ID).expect("buyer one uuid"),
    );
    token_to_user_id.insert(
        "buyer-two-token".to_string(),
        Uuid::parse_str(TEST_BUYER_TWO_ID).expect("buyer two uuid"),
    );

    let (auth_base_url, auth_handle) = spawn_mock_auth_server(token_to_user_id, None).await;
    let app = test_app_with_auth(auth_base_url);
    let order_id = first_order_id_with_bearer(&app, "buyer-one-token").await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/orders/{}/confirm", order_id))
                .header("authorization", "Bearer buyer-two-token")
                .header("content-type", "application/json")
                .body(Body::from(json!({}).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    auth_handle.abort();
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
async fn create_dispute_returns_forbidden_for_non_buyer_when_auth_enabled() {
    let mut token_to_user_id = HashMap::new();
    token_to_user_id.insert(
        "buyer-one-token".to_string(),
        Uuid::parse_str(TEST_BUYER_ONE_ID).expect("buyer one uuid"),
    );
    token_to_user_id.insert(
        "buyer-two-token".to_string(),
        Uuid::parse_str(TEST_BUYER_TWO_ID).expect("buyer two uuid"),
    );

    let (auth_base_url, auth_handle) = spawn_mock_auth_server(token_to_user_id, None).await;
    let app = test_app_with_auth(auth_base_url);
    let order_id = first_order_id_with_bearer(&app, "buyer-one-token").await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/orders/{}/dispute/new", order_id))
                .header("authorization", "Bearer buyer-two-token")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "reason": "Barang tidak sesuai dengan foto",
                        "details": "Attempt by non-owner"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    auth_handle.abort();
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
    let payload = json!({ "actorId": TEST_BUYER_ONE_ID }).to_string();

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

    let in_transit_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/seller/orders/{}/shipping", order_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "status": "in_transit",
                        "tracking": "TRK-CNF-1"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(in_transit_response.status(), StatusCode::NO_CONTENT);

    let delivered_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/seller/orders/{}/shipping", order_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "status": "delivered",
                        "tracking": "TRK-CNF-1"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(delivered_response.status(), StatusCode::NO_CONTENT);

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
async fn confirm_order_returns_conflict_when_order_not_ready() {
    let app = test_app();
    let order_id = first_order_id(&app).await;

    let confirm_response = app
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

    assert_eq!(confirm_response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn create_dispute_updates_order_state_and_tags() {
    let app = test_app();
    let order_id = first_order_id(&app).await;
    let reason = "Barang tidak sesuai dengan foto";
    let details = "Warna berbeda dan ada goresan";

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
                        "tracking": "TRK-DSP-1"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(shipping_response.status(), StatusCode::NO_CONTENT);

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
async fn create_dispute_rejects_completed_order() {
    let app = test_app();
    let order_id = first_order_id(&app).await;

    let in_transit_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/seller/orders/{}/shipping", order_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "status": "in_transit",
                        "tracking": "TRK-DSP-2"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(in_transit_response.status(), StatusCode::NO_CONTENT);

    let delivered_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/seller/orders/{}/shipping", order_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "status": "delivered",
                        "tracking": "TRK-DSP-2"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(delivered_response.status(), StatusCode::NO_CONTENT);

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

    let dispute_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/orders/{}/dispute/new", order_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "reporter_id": "buyer-vel",
                        "reason": "Barang tidak sesuai dengan foto",
                        "details": "Testing terminal transition"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(dispute_response.status(), StatusCode::CONFLICT);
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
async fn update_shipping_rejects_backward_transition_after_delivery() {
    let app = test_app();
    let order_id = first_order_id(&app).await;

    let in_transit_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/seller/orders/{}/shipping", order_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "status": "in_transit",
                        "tracking": "TRK-BWD-1"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(in_transit_response.status(), StatusCode::NO_CONTENT);

    let delivered_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/seller/orders/{}/shipping", order_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "status": "delivered",
                        "tracking": "TRK-BWD-1"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(delivered_response.status(), StatusCode::NO_CONTENT);

    let backward_response = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/seller/orders/{}/shipping", order_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "status": "in_transit",
                        "tracking": "TRK-BWD-1"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(backward_response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn publish_event_adds_notification_for_target_user() {
    let app = test_app();
    let before = list_notifications_by_user(&app, TEST_BUYER_ONE_ID).await;
    let before_count = before["data"].as_array().expect("data").len();
    let expected_title = format!("Order updated for {}", TEST_BUYER_ONE_ID);

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
                        "title": expected_title.clone(),
                        "body": "Your order status moved to processing.",
                        "channel": "inbox",
                        "metadata": {
                            "userId": TEST_BUYER_ONE_ID
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(publish_response.status(), StatusCode::NO_CONTENT);

    let after = list_notifications_by_user(&app, TEST_BUYER_ONE_ID).await;
    let after_data = after["data"].as_array().expect("data");
    assert_eq!(after_data.len(), before_count + 1);
    assert!(after_data.iter().any(|notification| {
        notification["title"]
            .as_str()
            .map(|title| title == expected_title.as_str())
            .unwrap_or(false)
    }));
}

#[tokio::test]
async fn publish_event_requires_internal_secret_when_configured() {
    let app = test_app_with_internal_secret("order-secret");
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/events/notifications")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "orderId": null,
                        "type": "OrderUpdate",
                        "title": "Unauthorized event",
                        "body": "This should not be accepted.",
                        "channel": "inbox",
                        "metadata": {
                            "userId": TEST_BUYER_ONE_ID
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn publish_event_accepts_valid_internal_secret_when_configured() {
    let app = test_app_with_internal_secret("order-secret");
    let before = list_notifications_by_user(&app, TEST_BUYER_ONE_ID).await;
    let before_count = before["data"].as_array().expect("data").len();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/events/notifications")
                .header("x-internal-secret", "order-secret")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "orderId": null,
                        "type": "OrderUpdate",
                        "title": "Authorized event",
                        "body": "This should be accepted.",
                        "channel": "inbox",
                        "metadata": {
                            "userId": TEST_BUYER_ONE_ID
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let after = list_notifications_by_user(&app, TEST_BUYER_ONE_ID).await;
    assert_eq!(
        after["data"].as_array().expect("data").len(),
        before_count + 1
    );
}

#[tokio::test]
async fn mark_notification_read_parallel_requests_are_consistent() {
    let app = test_app();
    let notification_id = first_notification_id(&app).await;
    let payload = json!({ "actorId": TEST_BUYER_ONE_ID }).to_string();

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

#[derive(Clone)]
struct ProfileTarget {
    name: &'static str,
    method: Method,
    path: String,
}

#[tokio::test]
#[ignore = "manual in-memory profiling; run with --ignored --nocapture"]
async fn profile_order_notifications_without_database() {
    let iterations = std::env::var("ORDER_PROFILE_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_IN_MEMORY_PROFILE_ITERATIONS);
    let warmup = std::env::var("ORDER_PROFILE_WARMUP")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_IN_MEMORY_PROFILE_WARMUP);
    let apdex_threshold_ms = std::env::var("ORDER_PROFILE_APDEX_MS")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value > 0.0)
        .unwrap_or(DEFAULT_IN_MEMORY_PROFILE_APDEX_MS);

    let app = test_app();
    let order_id = first_order_id(&app).await;
    let notification_id = first_notification_id(&app).await;
    let targets = vec![
        ProfileTarget {
            name: "buyer_orders_all",
            method: Method::GET,
            path: "/orders".to_string(),
        },
        ProfileTarget {
            name: "buyer_orders_active",
            method: Method::GET,
            path: "/orders?stage=active".to_string(),
        },
        ProfileTarget {
            name: "buyer_orders_processing",
            method: Method::GET,
            path: "/orders?stage=processing".to_string(),
        },
        ProfileTarget {
            name: "seller_orders_all",
            method: Method::GET,
            path: "/seller/orders".to_string(),
        },
        ProfileTarget {
            name: "buyer_order_detail",
            method: Method::GET,
            path: format!("/orders/{order_id}"),
        },
        ProfileTarget {
            name: "seller_order_detail",
            method: Method::GET,
            path: format!("/seller/orders/{order_id}"),
        },
        ProfileTarget {
            name: "notifications_all",
            method: Method::GET,
            path: "/notifications?limit=20".to_string(),
        },
        ProfileTarget {
            name: "notifications_unread",
            method: Method::GET,
            path: "/notifications?limit=20&unreadOnly=true".to_string(),
        },
        ProfileTarget {
            name: "notification_detail",
            method: Method::GET,
            path: format!("/notifications/{notification_id}"),
        },
    ];

    for target in &targets {
        for _ in 0..warmup {
            profile_request(&app, target).await;
        }
    }

    let mut summaries = Vec::new();
    for target in &targets {
        let mut latencies = Vec::with_capacity(iterations);
        let mut success_count = 0usize;

        for _ in 0..iterations {
            let result = profile_request(&app, target).await;
            latencies.push(result.elapsed_ms);
            if result.status.is_success() {
                success_count += 1;
            }
        }

        summaries.push(build_profile_summary(
            target,
            &latencies,
            success_count,
            apdex_threshold_ms,
        ));
    }

    println!(
        "in-memory order/notification profile: iterations={iterations}, warmup={warmup}, apdex_t={apdex_threshold_ms}ms"
    );
    for summary in &summaries {
        println!(
            "{:<28} count={:<4} errors={:<4} avg={:<8.3} p50={:<8.3} p95={:<8.3} p99={:<8.3} apdex={:<5.3}",
            summary["endpoint"].as_str().unwrap_or_default(),
            summary["count"].as_u64().unwrap_or_default(),
            summary["errorCount"].as_u64().unwrap_or_default(),
            summary["averageMs"].as_f64().unwrap_or_default(),
            summary["p50Ms"].as_f64().unwrap_or_default(),
            summary["p95Ms"].as_f64().unwrap_or_default(),
            summary["p99Ms"].as_f64().unwrap_or_default(),
            summary["apdex"].as_f64().unwrap_or_default(),
        );
    }

    fs::create_dir_all("performance/results").expect("create performance results directory");
    let output_path = format!(
        "performance/results/in-memory-order-notification-profile-{}.json",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let report = json!({
        "profileType": "in-memory-controller",
        "database": "not used",
        "iterations": iterations,
        "warmup": warmup,
        "apdexSatisfiedMs": apdex_threshold_ms,
        "orderId": order_id,
        "notificationId": notification_id,
        "summary": summaries,
    });
    fs::write(
        &output_path,
        serde_json::to_string_pretty(&report).expect("serialize profile report"),
    )
    .expect("write profile report");
    println!("wrote {output_path}");
}

struct ProfileRequestResult {
    status: StatusCode,
    elapsed_ms: f64,
}

async fn profile_request(app: &axum::Router, target: &ProfileTarget) -> ProfileRequestResult {
    let request = Request::builder()
        .method(target.method.clone())
        .uri(&target.path)
        .body(Body::empty())
        .expect("profile request");

    let started_at = Instant::now();
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("profile response");
    let status = response.status();
    let _ = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("consume profile body");
    let elapsed_ms = started_at.elapsed().as_secs_f64() * 1000.0;

    ProfileRequestResult { status, elapsed_ms }
}

fn build_profile_summary(
    target: &ProfileTarget,
    latencies: &[f64],
    success_count: usize,
    apdex_threshold_ms: f64,
) -> Value {
    let count = latencies.len();
    let average_ms = if count == 0 {
        0.0
    } else {
        latencies.iter().sum::<f64>() / count as f64
    };
    let satisfied = latencies
        .iter()
        .filter(|value| **value <= apdex_threshold_ms)
        .count();
    let tolerated = latencies
        .iter()
        .filter(|value| **value > apdex_threshold_ms && **value <= apdex_threshold_ms * 4.0)
        .count();
    let apdex = if count == 0 {
        0.0
    } else {
        (satisfied as f64 + tolerated as f64 / 2.0) / count as f64
    };

    json!({
        "endpoint": target.name,
        "method": target.method.as_str(),
        "path": target.path,
        "count": count,
        "successCount": success_count,
        "errorCount": count.saturating_sub(success_count),
        "averageMs": round_ms(average_ms),
        "p50Ms": round_ms(percentile(latencies, 50.0)),
        "p95Ms": round_ms(percentile(latencies, 95.0)),
        "p99Ms": round_ms(percentile(latencies, 99.0)),
        "maxMs": round_ms(latencies.iter().copied().fold(0.0, f64::max)),
        "apdex": (apdex * 1000.0).round() / 1000.0,
    })
}

fn percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let index = ((percentile / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[index.saturating_sub(1).min(sorted.len() - 1)]
}

fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

#[derive(Clone)]
struct DbProfileSeed {
    buyer_id: Uuid,
    seller_id: Uuid,
    order_id: Uuid,
    notification_id: Uuid,
}

fn test_app_with_db_pool(pool: sqlx::postgres::PgPool) -> axum::Router {
    let state = AppState {
        auth_base_url: String::new(),
        auth_http_client: reqwest::Client::new(),
        order_repo: OrderRepositoryHandle::new(DbOrderRepository::new(pool.clone())),
        notification_repo: NotificationRepositoryHandle::new(DbNotificationRepository::new(pool)),
        internal_secret: None,
    };
    create_router(state)
}

#[tokio::test]
#[ignore = "manual DB-backed profiling; requires APP_DATABASE_URL pointing to a disposable profiling database"]
async fn profile_order_notifications_with_database() {
    let database_url = std::env::var("APP_DATABASE_URL")
        .expect("APP_DATABASE_URL must point to a disposable profiling database");
    let iterations = std::env::var("ORDER_PROFILE_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_IN_MEMORY_PROFILE_ITERATIONS);
    let warmup = std::env::var("ORDER_PROFILE_WARMUP")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_IN_MEMORY_PROFILE_WARMUP);
    let apdex_threshold_ms = std::env::var("ORDER_PROFILE_APDEX_MS")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value > 0.0)
        .unwrap_or(500.0);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect profiling database");
    let seed = seed_db_profile_data(&pool)
        .await
        .expect("seed profiling database");
    let app = test_app_with_db_pool(pool.clone());

    let targets = vec![
        ProfileTarget {
            name: "buyer_orders_all",
            method: Method::GET,
            path: format!("/orders?userId={}", seed.buyer_id),
        },
        ProfileTarget {
            name: "buyer_orders_processing",
            method: Method::GET,
            path: format!("/orders?userId={}&stage=processing", seed.buyer_id),
        },
        ProfileTarget {
            name: "seller_orders_all",
            method: Method::GET,
            path: format!("/seller/orders?userId={}", seed.seller_id),
        },
        ProfileTarget {
            name: "buyer_order_detail",
            method: Method::GET,
            path: format!("/orders/{}", seed.order_id),
        },
        ProfileTarget {
            name: "seller_order_detail",
            method: Method::GET,
            path: format!("/seller/orders/{}", seed.order_id),
        },
        ProfileTarget {
            name: "notifications_all",
            method: Method::GET,
            path: format!("/notifications?userId={}&limit=20", seed.buyer_id),
        },
        ProfileTarget {
            name: "notifications_unread",
            method: Method::GET,
            path: format!(
                "/notifications?userId={}&limit=20&unreadOnly=true",
                seed.buyer_id
            ),
        },
        ProfileTarget {
            name: "notification_detail",
            method: Method::GET,
            path: format!("/notifications/{}", seed.notification_id),
        },
    ];

    for target in &targets {
        for _ in 0..warmup {
            profile_request(&app, target).await;
        }
    }

    let mut summaries = Vec::new();
    for target in &targets {
        let mut latencies = Vec::with_capacity(iterations);
        let mut success_count = 0usize;

        for _ in 0..iterations {
            let result = profile_request(&app, target).await;
            latencies.push(result.elapsed_ms);
            if result.status.is_success() {
                success_count += 1;
            }
        }

        summaries.push(build_profile_summary(
            target,
            &latencies,
            success_count,
            apdex_threshold_ms,
        ));
    }

    let explain = explain_db_profile_queries(&pool, &seed)
        .await
        .expect("run explain analyze");

    println!(
        "DB-backed order/notification profile: iterations={iterations}, warmup={warmup}, apdex_t={apdex_threshold_ms}ms"
    );
    for summary in &summaries {
        println!(
            "{:<28} count={:<4} errors={:<4} avg={:<8.3} p50={:<8.3} p95={:<8.3} p99={:<8.3} apdex={:<5.3}",
            summary["endpoint"].as_str().unwrap_or_default(),
            summary["count"].as_u64().unwrap_or_default(),
            summary["errorCount"].as_u64().unwrap_or_default(),
            summary["averageMs"].as_f64().unwrap_or_default(),
            summary["p50Ms"].as_f64().unwrap_or_default(),
            summary["p95Ms"].as_f64().unwrap_or_default(),
            summary["p99Ms"].as_f64().unwrap_or_default(),
            summary["apdex"].as_f64().unwrap_or_default(),
        );
    }

    fs::create_dir_all("performance/results").expect("create performance results directory");
    let output_path = format!(
        "performance/results/db-backed-order-notification-profile-{}.json",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let report = json!({
        "profileType": "db-backed-controller",
        "database": "external profiling database",
        "iterations": iterations,
        "warmup": warmup,
        "apdexSatisfiedMs": apdex_threshold_ms,
        "buyerId": seed.buyer_id,
        "sellerId": seed.seller_id,
        "orderId": seed.order_id,
        "notificationId": seed.notification_id,
        "summary": summaries,
        "explainAnalyze": explain,
    });
    fs::write(
        &output_path,
        serde_json::to_string_pretty(&report).expect("serialize DB profile report"),
    )
    .expect("write DB profile report");
    println!("wrote {output_path}");
}

async fn seed_db_profile_data(pool: &sqlx::postgres::PgPool) -> Result<DbProfileSeed, sqlx::Error> {
    let buyer_id = Uuid::parse_str(PROFILE_BUYER_ID).expect("profile buyer uuid");
    let seller_id = Uuid::parse_str(PROFILE_SELLER_ID).expect("profile seller uuid");
    let listing_id =
        Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").expect("profile listing uuid");
    let auction_id =
        Uuid::parse_str("dddddddd-dddd-dddd-dddd-dddddddddddd").expect("profile auction uuid");
    let order_id =
        Uuid::parse_str("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee").expect("profile order uuid");
    let notification_id =
        Uuid::parse_str("ffffffff-ffff-ffff-ffff-ffffffffffff").expect("profile notification uuid");

    let category_id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO categories (name, slug, image_url)
        VALUES ('Performance Profiling', $1, '')
        ON CONFLICT (slug)
        DO UPDATE SET updated_at = CURRENT_TIMESTAMP
        RETURNING id
        "#,
    )
    .bind(PROFILE_CATEGORY_SLUG)
    .fetch_one(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO listings (
            id,
            seller_id,
            seller_name,
            category_id,
            category_name,
            title,
            description,
            start_price,
            reserve_price,
            current_price,
            min_increment,
            bid_count,
            status,
            auction_id,
            starts_at,
            ends_at
        )
        VALUES (
            $1, $2, 'Profiling Seller', $3, 'Performance Profiling',
            'Profiling Lot - Order Module', 'Synthetic listing for order profiling',
            100000, 120000, 150000, 1000, 3, 'SOLD'::listing_status,
            NULL, NOW() - INTERVAL '2 days', NOW() + INTERVAL '1 day'
        )
        ON CONFLICT (id)
        DO UPDATE SET
            seller_id = EXCLUDED.seller_id,
            seller_name = EXCLUDED.seller_name,
            category_id = EXCLUDED.category_id,
            category_name = EXCLUDED.category_name,
            title = EXCLUDED.title,
            status = EXCLUDED.status,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(listing_id)
    .bind(seller_id)
    .bind(category_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO auctions (
            id,
            listing_id,
            seller_id,
            seller_name,
            title,
            description,
            image_url,
            start_price,
            current_price,
            reserve_price,
            bid_increment,
            bid_count,
            status,
            winner_id,
            winner_name,
            starts_at,
            ends_at,
            original_ends_at
        )
        VALUES (
            $1, $2, $3, 'Profiling Seller',
            'Profiling Lot - Order Module', 'Synthetic auction for order profiling', '',
            100000, 150000, 120000, 1000, 3, 'WON'::auction_status,
            $4, 'Profiling Buyer',
            NOW() - INTERVAL '2 days', NOW() - INTERVAL '1 day', NOW() - INTERVAL '1 day'
        )
        ON CONFLICT (id)
        DO UPDATE SET
            listing_id = EXCLUDED.listing_id,
            seller_id = EXCLUDED.seller_id,
            winner_id = EXCLUDED.winner_id,
            status = EXCLUDED.status
        "#,
    )
    .bind(auction_id)
    .bind(listing_id)
    .bind(seller_id)
    .bind(buyer_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        UPDATE listings
        SET auction_id = $2,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $1
        "#,
    )
    .bind(listing_id)
    .bind(auction_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO orders (
            id,
            auction_id,
            listing_id,
            buyer_id,
            buyer_name,
            seller_id,
            seller_name,
            title,
            image_url,
            final_price,
            status,
            shipping_status,
            carrier,
            tracking_number,
            paid_at,
            shipped_at,
            delivered_at,
            is_disputed
        )
        VALUES (
            $1, $2, $3, $4, 'Profiling Buyer', $5, 'Profiling Seller',
            'Profiling Lot - Order Module', '', 150000,
            'DELIVERED'::order_status, 'DELIVERED'::shipping_status,
            'Synthetic Courier', 'PROFILE-TRACK-001',
            NOW() - INTERVAL '1 day',
            NOW() - INTERVAL '12 hours',
            NOW() - INTERVAL '1 hour',
            FALSE
        )
        ON CONFLICT (id)
        DO UPDATE SET
            status = EXCLUDED.status,
            shipping_status = EXCLUDED.shipping_status,
            carrier = EXCLUDED.carrier,
            tracking_number = EXCLUDED.tracking_number,
            is_disputed = FALSE,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(order_id)
    .bind(auction_id)
    .bind(listing_id)
    .bind(buyer_id)
    .bind(seller_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO notifications (
            id,
            user_id,
            type,
            title,
            message,
            is_read,
            reference_id,
            reference_type
        )
        VALUES
            ($1, $2, 'ORDER_DELIVERED'::notification_type, 'Profiling order delivered',
             'Synthetic notification for profiling.', FALSE, $3, 'order'::reference_type),
            ('99999999-9999-9999-9999-999999999999'::uuid, $2, 'ORDER_SHIPPED'::notification_type,
             'Profiling order shipped', 'Synthetic read notification for profiling.', TRUE, $3,
             'order'::reference_type)
        ON CONFLICT (id)
        DO UPDATE SET
            user_id = EXCLUDED.user_id,
            type = EXCLUDED.type,
            title = EXCLUDED.title,
            message = EXCLUDED.message,
            is_read = EXCLUDED.is_read,
            reference_id = EXCLUDED.reference_id,
            reference_type = EXCLUDED.reference_type
        "#,
    )
    .bind(notification_id)
    .bind(buyer_id)
    .bind(order_id)
    .execute(pool)
    .await?;

    Ok(DbProfileSeed {
        buyer_id,
        seller_id,
        order_id,
        notification_id,
    })
}

async fn explain_db_profile_queries(
    pool: &sqlx::postgres::PgPool,
    seed: &DbProfileSeed,
) -> Result<Value, sqlx::Error> {
    let buyer_orders = explain_query(
        pool,
        "buyer_order_list",
        r#"
        EXPLAIN (ANALYZE, BUFFERS)
        SELECT id, title, buyer_id, seller_id, final_price, status::text AS status_text,
               shipping_status::text AS shipping_status_text, carrier, tracking_number,
               is_disputed, created_at, updated_at
        FROM orders
        WHERE buyer_id = $1
        ORDER BY updated_at DESC
        "#,
        seed.buyer_id,
    )
    .await?;
    let seller_orders = explain_query(
        pool,
        "seller_order_list",
        r#"
        EXPLAIN (ANALYZE, BUFFERS)
        SELECT id, title, buyer_id, seller_id, final_price, status::text AS status_text,
               shipping_status::text AS shipping_status_text, carrier, tracking_number,
               is_disputed, created_at, updated_at
        FROM orders
        WHERE seller_id = $1
        ORDER BY updated_at DESC
        "#,
        seed.seller_id,
    )
    .await?;
    let notification_list = explain_query(
        pool,
        "notification_unread_list",
        r#"
        EXPLAIN (ANALYZE, BUFFERS)
        SELECT id, user_id, type::text AS type_text, title, message, reference_id,
               reference_type::text AS reference_type_text, created_at, read_at
        FROM notifications
        WHERE user_id = $1 AND is_read = FALSE
        ORDER BY created_at DESC
        LIMIT 20
        "#,
        seed.buyer_id,
    )
    .await?;

    Ok(json!([buyer_orders, seller_orders, notification_list]))
}

async fn explain_query(
    pool: &sqlx::postgres::PgPool,
    label: &str,
    sql: &str,
    id: Uuid,
) -> Result<Value, sqlx::Error> {
    let plan = sqlx::query_scalar::<_, String>(sql)
        .bind(id)
        .fetch_all(pool)
        .await?;
    Ok(json!({
        "label": label,
        "plan": plan,
    }))
}
