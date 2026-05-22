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

use super::support::create_app_state;

const TEST_BUYER_ONE_ID: &str = "11111111-1111-1111-1111-111111111111";
const TEST_SELLER_ONE_ID: &str = "22222222-2222-2222-2222-222222222222";
const TEST_BUYER_TWO_ID: &str = "33333333-3333-3333-3333-333333333333";
const DEFAULT_IN_MEMORY_PROFILE_ITERATIONS: usize = 200;
const DEFAULT_IN_MEMORY_PROFILE_WARMUP: usize = 20;
const DEFAULT_IN_MEMORY_PROFILE_APDEX_MS: f64 = 10.0;

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
