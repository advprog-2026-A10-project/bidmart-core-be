use std::sync::Arc;
use tokio::sync::Mutex;

use crate::modules::order::domain::entities::{Order, OrderId, OrderStage, OrderStatus};
use crate::modules::order::domain::errors::OrderError;
use crate::modules::order::domain::traits::OrderRepository;
use chrono::Utc;

pub struct InMemoryOrderRepository {
    orders: Mutex<Vec<Order>>,
}

impl InMemoryOrderRepository {
    pub fn new(orders: Vec<Order>) -> Self {
        Self {
            orders: Mutex::new(orders),
        }
    }
}

#[async_trait::async_trait]
impl OrderRepository for Arc<InMemoryOrderRepository> {
    async fn list_orders(
        &self,
        role: &str,
        user_id: Option<&str>,
        stage: Option<OrderStage>,
    ) -> Result<Vec<Order>, OrderError> {
        let orders = self.orders.lock().await;
        let filtered = orders
            .iter()
            .filter(|order| match role {
                "seller" => user_id.map_or(true, |id| order.seller_id == id),
                _ => user_id.map_or(true, |id| order.buyer_id == id),
            })
            .filter(|order| stage.as_ref().map_or(true, |s| order.stage == *s))
            .cloned()
            .collect();
        Ok(filtered)
    }

    async fn get_order(&self, order_id: OrderId) -> Result<Order, OrderError> {
        let orders = self.orders.lock().await;
        orders
            .iter()
            .find(|order| order.id == order_id)
            .cloned()
            .ok_or(OrderError::NotFound)
    }

    async fn confirm_order(&self, order_id: OrderId, _actor_id: &str) -> Result<(), OrderError> {
        let mut orders = self.orders.lock().await;
        let order = orders
            .iter_mut()
            .find(|order| order.id == order_id)
            .ok_or(OrderError::NotFound)?;

        if order.status == OrderStatus::Delivered {
            return Err(OrderError::InvalidTransition);
        }

        order.status = OrderStatus::Delivered;
        order.stage = OrderStage::Completed;
        order.updated_at = Utc::now().to_rfc3339();
        order.last_activity = "Buyer confirmed receipt".into();
        Ok(())
    }

    async fn create_dispute(
        &self,
        order_id: OrderId,
        _reporter_id: &str,
        reason: &str,
        details: Option<&str>,
    ) -> Result<(), OrderError> {
        let mut orders = self.orders.lock().await;
        let order = orders
            .iter_mut()
            .find(|order| order.id == order_id)
            .ok_or(OrderError::NotFound)?;

        order.status = OrderStatus::DisputeAlert;
        order.stage = OrderStage::Processing;
        order.updated_at = Utc::now().to_rfc3339();
        order.last_activity = "Dispute opened by buyer".into();
        order
            .tags
            .push(format!("Dispute reason: {}", reason.trim()));
        if let Some(extra_details) = details.map(str::trim).filter(|value| !value.is_empty()) {
            order.tags.push(format!("Dispute details: {}", extra_details));
        }
        Ok(())
    }

    async fn update_shipping_status(
        &self,
        order_id: OrderId,
        status: &str,
        tracking: Option<&str>,
    ) -> Result<(), OrderError> {
        let mut orders = self.orders.lock().await;
        let order = orders
            .iter_mut()
            .find(|order| order.id == order_id)
            .ok_or(OrderError::NotFound)?;

        let normalized = status.trim().to_lowercase();
        match normalized.as_str() {
            "in_transit" | "in transit" | "shipped" | "packed" => {
                order.status = OrderStatus::InTransit;
                order.stage = OrderStage::Processing;
                order.last_activity = "Seller updated shipment status".into();
            }
            "needs_confirmation" | "needs confirmation" | "delivered" => {
                order.status = OrderStatus::NeedsConfirmation;
                order.stage = OrderStage::Processing;
                order.last_activity = "Package delivered, waiting buyer confirmation".into();
            }
            _ => return Err(OrderError::InvalidTransition),
        }

        if let Some(code) = tracking.map(str::trim).filter(|value| !value.is_empty()) {
            order.tags.retain(|tag| !tag.starts_with("Tracking: "));
            order.tags.push(format!("Tracking: {}", code));
        }

        order.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    async fn record_event(
        &self,
        order_id: OrderId,
        event: &str,
        _metadata: Option<serde_json::Value>,
    ) -> Result<(), OrderError> {
        let mut orders = self.orders.lock().await;
        let order = orders
            .iter_mut()
            .find(|order| order.id == order_id)
            .ok_or(OrderError::NotFound)?;

        order.updated_at = Utc::now().to_rfc3339();
        order.last_activity = event.to_string();
        Ok(())
    }
}
