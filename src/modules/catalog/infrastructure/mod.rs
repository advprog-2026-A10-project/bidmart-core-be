use axum::{
    middleware::{from_fn, from_fn_with_state},
    routing, Router,
};
use reqwest::Client;
use sqlx::postgres::PgPool;
use std::sync::Arc;

pub mod controllers;
pub mod middleware;
pub mod repositories;
pub mod services;

use controllers::{buyer_controller, category_controller, internal_controller, seller_controller};
use repositories::{
    PostgresAuctionLifecycleRepository, PostgresCategoryRepository, PostgresListingImageRepository,
    PostgresListingIntegrationRepository, PostgresListingRepository,
};
use services::ListingIntegrationService;

use crate::modules::catalog::application::use_cases::{
    buyer_listing_use_cases::BuyerListingUseCases, category_use_cases::CategoryUseCases,
    listing_use_cases::ListingUseCases,
};
use crate::modules::catalog::domain::traits::{AuctionLifecyclePort, ListingIntegrationPort};

#[derive(Clone)]
pub struct AppState {
    pub listing_use_cases: Arc<ListingUseCases>,
    pub buyer_listing_use_cases: Arc<BuyerListingUseCases>,
    pub category_use_cases: Arc<CategoryUseCases>,
    pub integration_service: Arc<dyn ListingIntegrationPort>,
    pub auth_base_url: String,
    pub auth_http_client: Client,
}

impl AppState {
    pub fn new(pool: PgPool, auth_base_url: String) -> Self {
        let listing_repo = Arc::new(PostgresListingRepository::new(pool.clone()));
        let image_repo = Arc::new(PostgresListingImageRepository::new(pool.clone()));
        let category_repo = Arc::new(PostgresCategoryRepository::new(pool.clone()));
        let integration_repo: Arc<dyn ListingIntegrationPort> =
            Arc::new(PostgresListingIntegrationRepository::new(pool.clone()));
        let auction_lifecycle: Arc<dyn AuctionLifecyclePort> =
            Arc::new(PostgresAuctionLifecycleRepository::new(pool.clone()));

        Self {
            listing_use_cases: Arc::new(ListingUseCases::new(
                listing_repo.clone(),
                image_repo.clone(),
                category_repo.clone(),
                auction_lifecycle,
            )),
            buyer_listing_use_cases: Arc::new(BuyerListingUseCases::new(
                listing_repo.clone(),
                image_repo.clone(),
                category_repo.clone(),
            )),
            category_use_cases: Arc::new(CategoryUseCases::new(category_repo)),
            integration_service: Arc::new(ListingIntegrationService::new(integration_repo)),
            auth_base_url,
            auth_http_client: Client::new(),
        }
    }
}

pub fn create_router(state: AppState) -> Router {
    let seller_routes = Router::new()
        .route(
            "/seller/listings",
            routing::get(seller_controller::list_my_listings)
                .post(seller_controller::create_listing),
        )
        .route(
            "/seller/listings/:id",
            routing::get(seller_controller::get_my_listing)
                .patch(seller_controller::update_listing)
                .delete(seller_controller::cancel_listing),
        )
        .route(
            "/seller/listings/:id/publish",
            routing::post(seller_controller::publish_listing),
        )
        .layer(from_fn_with_state(state.clone(), middleware::require_auth));

    let buyer_routes = Router::new()
        .route("/catalog", routing::get(buyer_controller::browse_catalog))
        .route(
            "/c/*category_path",
            routing::get(buyer_controller::browse_by_category_path),
        )
        .route(
            "/listings/:id",
            routing::get(buyer_controller::get_public_listing),
        );

    let category_routes = Router::new()
        .route(
            "/categories",
            routing::get(category_controller::list_categories)
                .post(category_controller::create_category),
        )
        .route(
            "/categories/slug/:slug",
            routing::get(category_controller::get_category_by_slug),
        )
        .route(
            "/categories/:id",
            routing::get(category_controller::get_category_by_id)
                .patch(category_controller::update_category)
                .delete(category_controller::delete_category),
        );

    let internal_routes = Router::new()
        .route(
            "/internal/listings/:id/status",
            routing::get(internal_controller::get_listing_status)
                .patch(internal_controller::update_status),
        )
        .route(
            "/internal/listings/:id/auction",
            routing::patch(internal_controller::link_auction),
        )
        .route(
            "/internal/listings/:id/ends-at",
            routing::patch(internal_controller::update_ends_at),
        );

    Router::new()
        .nest(
            "/api/v1",
            seller_routes
                .merge(buyer_routes)
                .merge(category_routes)
                .merge(internal_routes),
        )
        // Per-module request tracer (scoped under `core_be.catalog.request`).
        // Outermost so it observes the final status + auth outcome.
        .layer(from_fn(middleware::request_trace))
        .with_state(state)
}
