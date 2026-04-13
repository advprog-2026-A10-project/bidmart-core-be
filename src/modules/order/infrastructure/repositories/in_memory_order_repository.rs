use std::sync::Arc;
use tokio::sync::Mutex;

use crate::modules::order::domain::entities::{Order, OrderId, OrderStage};
use crate::modules::order::domain::errors::OrderError;
use crate::modules::order::domain::traits::OrderRepository;

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
        _role: &str,
        _user_id: Option<&str>,
        stage: Option<OrderStage>,
    ) -> Result<Vec<Order>, OrderError> {
        let orders = self.orders.lock().await;
        let filtered = orders
            .iter()
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

    async fn confirm_order(&self, _order_id: OrderId, _actor_id: &str) -> Result<(), OrderError> {
        Ok(())
    }

    async fn create_dispute(
        &self,
        _order_id: OrderId,
        _reporter_id: &str,
        _reason: &str,
    ) -> Result<(), OrderError> {
        Ok(())
    }

    async fn update_shipping_status(
        &self,
        _order_id: OrderId,
        _status: &str,
        _tracking: Option<&str>,
    ) -> Result<(), OrderError> {
        Ok(())
    }

    async fn record_event(
        &self,
        _order_id: OrderId,
        _event: &str,
        _metadata: Option<serde_json::Value>,
    ) -> Result<(), OrderError> {
        Ok(())
    }
}
