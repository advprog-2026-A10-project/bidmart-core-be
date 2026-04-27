use std::sync::Arc;
use tokio::sync::Mutex;

use crate::modules::order::domain::entities::{
    Notification, NotificationEventPayload, NotificationId,
};
use crate::modules::order::domain::errors::NotificationError;
use crate::modules::order::domain::traits::NotificationRepository;
use chrono::Utc;

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
        user_id: Option<&str>,
        limit: Option<u32>,
        unread_only: bool,
    ) -> Result<Vec<Notification>, NotificationError> {
        let notifications = self.notifications.lock().await;
        let iter = notifications.iter().filter(|notification| {
            let user_match = user_id.map_or(true, |id| {
                notification
                    .metadata
                    .as_ref()
                    .and_then(|metadata| metadata.get("userId"))
                    .and_then(|value| value.as_str())
                    .map_or(false, |meta_id| meta_id == id)
            });
            let unread_match = !unread_only || notification.read_at.is_none();
            user_match && unread_match
        });

        let result: Vec<Notification> = match limit {
            Some(limit_value) => iter.take(limit_value as usize).cloned().collect(),
            None => iter.cloned().collect(),
        };

        Ok(result)
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
        notification_id: NotificationId,
        _actor_id: &str,
    ) -> Result<(), NotificationError> {
        let mut notifications = self.notifications.lock().await;
        let notification = notifications
            .iter_mut()
            .find(|notification| notification.id == notification_id)
            .ok_or(NotificationError::NotFound)?;

        if notification.read_at.is_some() {
            return Err(NotificationError::AlreadyRead);
        }

        notification.read_at = Some(Utc::now().to_rfc3339());
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
