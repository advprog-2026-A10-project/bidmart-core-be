use std::sync::Arc;
use tokio::sync::Mutex;

use crate::modules::order::domain::entities::{
    Notification, NotificationEventPayload, NotificationId,
};
use crate::modules::order::domain::errors::NotificationError;
use crate::modules::order::domain::traits::NotificationRepository;

pub struct InMemoryNotificationRepository {
    notifications: Mutex<Vec<Notification>>,
}

impl InMemoryNotificationRepository {
    pub fn new(notifications: Vec<Notification>) -> Self {
        Self {
            notifications: Mutex::new(notifications),
        }
    }
}

#[async_trait::async_trait]
impl NotificationRepository for Arc<InMemoryNotificationRepository> {
    async fn list_notifications(
        &self,
        _user_id: Option<&str>,
        _limit: Option<u32>,
        _unread_only: bool,
    ) -> Result<Vec<Notification>, NotificationError> {
        let notifications = self.notifications.lock().await;
        Ok(notifications.clone())
    }

    async fn get_notification(
        &self,
        notification_id: NotificationId,
    ) -> Result<Notification, NotificationError> {
        let notifications = self.notifications.lock().await;
        notifications
            .iter()
            .find(|notification| notification.id == notification_id)
            .cloned()
            .ok_or(NotificationError::NotFound)
    }

    async fn mark_as_read(
        &self,
        _notification_id: NotificationId,
        _actor_id: &str,
    ) -> Result<(), NotificationError> {
        Ok(())
    }

    async fn publish_event(
        &self,
        payload: NotificationEventPayload,
    ) -> Result<(), NotificationError> {
        let mut notifications = self.notifications.lock().await;
        notifications.push(Notification {
            id: uuid::Uuid::new_v4(),
            order_id: payload.order_id,
            title: payload.title,
            body: payload.body,
            channel: payload.channel,
            notification_type: payload.notification_type,
            created_at: chrono::Utc::now().to_rfc3339(),
            read_at: None,
            metadata: payload.metadata,
        });
        Ok(())
    }
}
