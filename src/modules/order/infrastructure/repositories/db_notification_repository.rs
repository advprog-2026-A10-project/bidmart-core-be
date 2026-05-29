use anyhow::anyhow;
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

use crate::modules::order::domain::entities::{
    Notification, NotificationChannel, NotificationEventPayload, NotificationId, NotificationType,
};
use crate::modules::order::domain::errors::NotificationError;
use crate::modules::order::domain::traits::NotificationRepository;

#[derive(Clone)]
pub struct DbNotificationRepository {
    pool: PgPool,
}

impl DbNotificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl NotificationRepository for DbNotificationRepository {
    async fn list_notifications(
        &self,
        user_id: Option<&str>,
        limit: Option<u32>,
        unread_only: bool,
    ) -> Result<Vec<Notification>, NotificationError> {
        let parsed_user_id = match user_id {
            Some(value) => match Uuid::parse_str(value) {
                Ok(parsed) => Some(parsed),
                Err(_) => return Ok(Vec::new()),
            },
            None => None,
        };
        let capped_limit = limit.map(i64::from).unwrap_or(50).min(200);

        let rows = match (parsed_user_id, unread_only) {
            (Some(user_id), true) => {
                sqlx::query(
                    r#"
                    SELECT
                        id,
                        user_id,
                        type::text AS type_text,
                        title,
                        message,
                        reference_id,
                        reference_type::text AS reference_type_text,
                        created_at,
                        read_at
                    FROM notifications
                    WHERE user_id = $1
                      AND is_read = FALSE
                    ORDER BY created_at DESC
                    LIMIT $2
                    "#,
                )
                .bind(user_id)
                .bind(capped_limit)
                .fetch_all(&self.pool)
                .await
            }
            (Some(user_id), false) => {
                sqlx::query(
                    r#"
                    SELECT
                        id,
                        user_id,
                        type::text AS type_text,
                        title,
                        message,
                        reference_id,
                        reference_type::text AS reference_type_text,
                        created_at,
                        read_at
                    FROM notifications
                    WHERE user_id = $1
                    ORDER BY created_at DESC
                    LIMIT $2
                    "#,
                )
                .bind(user_id)
                .bind(capped_limit)
                .fetch_all(&self.pool)
                .await
            }
            (None, true) => {
                sqlx::query(
                    r#"
                    SELECT
                        id,
                        user_id,
                        type::text AS type_text,
                        title,
                        message,
                        reference_id,
                        reference_type::text AS reference_type_text,
                        created_at,
                        read_at
                    FROM notifications
                    WHERE is_read = FALSE
                    ORDER BY created_at DESC
                    LIMIT $1
                    "#,
                )
                .bind(capped_limit)
                .fetch_all(&self.pool)
                .await
            }
            (None, false) => {
                sqlx::query(
                    r#"
                    SELECT
                        id,
                        user_id,
                        type::text AS type_text,
                        title,
                        message,
                        reference_id,
                        reference_type::text AS reference_type_text,
                        created_at,
                        read_at
                    FROM notifications
                    ORDER BY created_at DESC
                    LIMIT $1
                    "#,
                )
                .bind(capped_limit)
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|error| NotificationError::Database(error.into()))?;

        rows.into_iter().map(map_row_to_notification).collect()
    }

    async fn get_notification(
        &self,
        notification_id: NotificationId,
    ) -> Result<Notification, NotificationError> {
        let row = sqlx::query(
            r#"
            SELECT
                id,
                user_id,
                type::text AS type_text,
                title,
                message,
                reference_id,
                reference_type::text AS reference_type_text,
                created_at,
                read_at
            FROM notifications
            WHERE id = $1
            "#,
        )
        .bind(notification_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| NotificationError::Database(error.into()))?
        .ok_or(NotificationError::NotFound)?;

        map_row_to_notification(row)
    }

    async fn mark_as_read(
        &self,
        notification_id: NotificationId,
        _actor_id: &str,
    ) -> Result<(), NotificationError> {
        let update = sqlx::query(
            r#"
            UPDATE notifications
            SET is_read = TRUE,
                read_at = COALESCE(read_at, CURRENT_TIMESTAMP)
            WHERE id = $1
              AND is_read = FALSE
            "#,
        )
        .bind(notification_id)
        .execute(&self.pool)
        .await
        .map_err(|error| NotificationError::Database(error.into()))?;

        if update.rows_affected() > 0 {
            return Ok(());
        }

        let current_state = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT is_read
            FROM notifications
            WHERE id = $1
            "#,
        )
        .bind(notification_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| NotificationError::Database(error.into()))?;

        match current_state {
            None => Err(NotificationError::NotFound),
            Some(true) => Err(NotificationError::AlreadyRead),
            Some(false) => Err(NotificationError::Database(anyhow!(
                "failed to mark notification as read"
            ))),
        }
    }

    async fn publish_event(
        &self,
        payload: NotificationEventPayload,
    ) -> Result<(), NotificationError> {
        let target_user_id = payload
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("userId"))
            .and_then(|value| value.as_str())
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or_else(|| {
                NotificationError::Database(anyhow!("metadata.userId must be a valid UUID string"))
            })?;

        let reference_id = payload.order_id;
        let reference_type = reference_id.map(|_| "order");

        sqlx::query(
            r#"
            INSERT INTO notifications (
                user_id,
                type,
                title,
                message,
                reference_id,
                reference_type
            )
            VALUES (
                $1,
                $2::notification_type,
                $3,
                $4,
                $5,
                $6::reference_type
            )
            "#,
        )
        .bind(target_user_id)
        .bind(payload.notification_type.as_db_value())
        .bind(payload.title)
        .bind(payload.body)
        .bind(reference_id)
        .bind(reference_type)
        .execute(&self.pool)
        .await
        .map_err(|error| NotificationError::Database(error.into()))?;

        Ok(())
    }
}

fn map_row_to_notification(row: PgRow) -> Result<Notification, NotificationError> {
    let id: Uuid = row.try_get("id").map_err(sqlx_error)?;
    let db_user_id: Uuid = row.try_get("user_id").map_err(sqlx_error)?;
    let raw_type: String = row.try_get("type_text").map_err(sqlx_error)?;
    let title: String = row.try_get("title").map_err(sqlx_error)?;
    let body: String = row.try_get("message").map_err(sqlx_error)?;
    let reference_id: Option<Uuid> = row.try_get("reference_id").map_err(sqlx_error)?;
    let reference_type: Option<String> = row.try_get("reference_type_text").map_err(sqlx_error)?;
    let created_at: DateTime<Utc> = row.try_get("created_at").map_err(sqlx_error)?;
    let read_at: Option<DateTime<Utc>> = row.try_get("read_at").map_err(sqlx_error)?;

    let notification_type = NotificationType::from_db_value(&raw_type).ok_or_else(|| {
        NotificationError::Database(anyhow!("unknown notification type from db: {raw_type}"))
    })?;

    let order_id = if reference_type.as_deref() == Some("order") {
        reference_id
    } else {
        None
    };

    Ok(Notification {
        id,
        order_id,
        title,
        body,
        channel: NotificationChannel::Inbox,
        notification_type,
        created_at: created_at.to_rfc3339(),
        read_at: read_at.map(|value| value.to_rfc3339()),
        metadata: Some(serde_json::json!({ "userId": db_user_id.to_string() })),
    })
}

fn sqlx_error(error: sqlx::Error) -> NotificationError {
    NotificationError::Database(error.into())
}
