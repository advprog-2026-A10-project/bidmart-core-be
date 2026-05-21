use anyhow::anyhow;
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

use crate::modules::order::domain::entities::{Order, OrderId, OrderStage, OrderStatus};
use crate::modules::order::domain::errors::OrderError;
use crate::modules::order::domain::traits::OrderRepository;

#[derive(Clone)]
pub struct DbOrderRepository {
    pool: PgPool,
}

impl DbOrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl OrderRepository for DbOrderRepository {
    async fn list_orders(
        &self,
        role: &str,
        user_id: Option<&str>,
        stage: Option<OrderStage>,
    ) -> Result<Vec<Order>, OrderError> {
        let parsed_user_id = match user_id {
            Some(value) => match Uuid::parse_str(value) {
                Ok(parsed) => Some(parsed),
                Err(_) => return Ok(Vec::new()),
            },
            None => None,
        };

        let rows = sqlx::query(
            r#"
            SELECT
                id,
                title,
                buyer_id,
                seller_id,
                final_price,
                status::text AS status_text,
                shipping_status::text AS shipping_status_text,
                carrier,
                tracking_number,
                is_disputed,
                created_at,
                updated_at
            FROM orders
            WHERE (
                $1 = 'seller'
                AND ($2::uuid IS NULL OR seller_id = $2)
            ) OR (
                $1 <> 'seller'
                AND ($2::uuid IS NULL OR buyer_id = $2)
            )
            ORDER BY updated_at DESC
            "#,
        )
        .bind(role)
        .bind(parsed_user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| OrderError::Database(error.into()))?;

        let mut orders = Vec::with_capacity(rows.len());
        for row in rows {
            let order = map_row_to_order(row)?;
            if stage
                .as_ref()
                .is_none_or(|expected| order.stage == *expected)
            {
                orders.push(order);
            }
        }
        Ok(orders)
    }

    async fn get_order(&self, order_id: OrderId) -> Result<Order, OrderError> {
        let row = sqlx::query(
            r#"
            SELECT
                id,
                title,
                buyer_id,
                seller_id,
                final_price,
                status::text AS status_text,
                shipping_status::text AS shipping_status_text,
                carrier,
                tracking_number,
                is_disputed,
                created_at,
                updated_at
            FROM orders
            WHERE id = $1
            "#,
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| OrderError::Database(error.into()))?
        .ok_or(OrderError::NotFound)?;

        map_row_to_order(row)
    }

    async fn confirm_order(&self, order_id: OrderId, _actor_id: &str) -> Result<(), OrderError> {
        let update = sqlx::query(
            r#"
            UPDATE orders
            SET
                status = 'CONFIRMED'::order_status,
                confirmed_at = COALESCE(confirmed_at, CURRENT_TIMESTAMP),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
              AND status = 'DELIVERED'::order_status
              AND is_disputed = FALSE
            "#,
        )
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(|error| OrderError::Database(error.into()))?;

        map_order_mutation_result(&self.pool, order_id, update.rows_affected()).await
    }

    async fn create_dispute(
        &self,
        order_id: OrderId,
        reporter_id: &str,
        reason: &str,
        details: Option<&str>,
    ) -> Result<(), OrderError> {
        // Auth integration extracts a real UUID from the validated session and
        // forwards it into the use-case. A free-form string should never reach
        // this layer — reject it as InvalidTransition (422-equivalent) instead
        // of silently writing a nil UUID into the disputes audit row.
        let reporter_uuid = Uuid::parse_str(reporter_id)
            .map_err(|_| OrderError::InvalidTransition)?;

        let reason_text = reason.trim();
        let details_text = details.map(str::trim).filter(|value| !value.is_empty());
        let description = match details_text {
            Some(extra) => format!("{}\nDetails: {}", reason_text, extra),
            None => reason_text.to_string(),
        };

        let update = sqlx::query(
            r#"
            UPDATE orders
            SET
                status = 'DISPUTED'::order_status,
                is_disputed = TRUE,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
              AND status IN (
                'SHIPPED'::order_status,
                'DELIVERED'::order_status,
                'DISPUTED'::order_status
              )
            "#,
        )
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(|error| OrderError::Database(error.into()))?;

        map_order_mutation_result(&self.pool, order_id, update.rows_affected()).await?;

        sqlx::query(
            r#"
            INSERT INTO disputes (
                order_id,
                opened_by,
                reason,
                description,
                status
            )
            VALUES (
                $1,
                $2,
                'ITEM_NOT_AS_DESCRIBED'::dispute_reason,
                $3,
                'OPEN'::dispute_status
            )
            ON CONFLICT (order_id)
            DO UPDATE
            SET
                opened_by = EXCLUDED.opened_by,
                reason = EXCLUDED.reason,
                description = EXCLUDED.description,
                status = 'OPEN'::dispute_status,
                resolved_at = NULL
            "#,
        )
        .bind(order_id)
        .bind(reporter_uuid)
        .bind(description)
        .execute(&self.pool)
        .await
        .map_err(|error| OrderError::Database(error.into()))?;

        Ok(())
    }

    async fn update_shipping_status(
        &self,
        order_id: OrderId,
        status: &str,
        tracking: Option<&str>,
    ) -> Result<(), OrderError> {
        let normalized = status.trim().to_lowercase();
        let (target_status, target_shipping_status, should_set_delivered_at) =
            match normalized.as_str() {
                "in_transit" | "in transit" | "shipped" | "packed" => ("SHIPPED", "SHIPPED", false),
                "needs_confirmation" | "needs confirmation" | "delivered" => {
                    ("DELIVERED", "DELIVERED", true)
                }
                _ => return Err(OrderError::InvalidTransition),
            };

        let tracking_number = tracking.map(str::trim).filter(|value| !value.is_empty());

        let update = sqlx::query(
            r#"
            UPDATE orders
            SET
                status = $2::order_status,
                shipping_status = $3::shipping_status,
                tracking_number = COALESCE($4, tracking_number),
                shipped_at = CASE
                    WHEN $3::shipping_status = 'SHIPPED'::shipping_status
                    THEN COALESCE(shipped_at, CURRENT_TIMESTAMP)
                    ELSE shipped_at
                END,
                delivered_at = CASE
                    WHEN $5::boolean = TRUE
                    THEN COALESCE(delivered_at, CURRENT_TIMESTAMP)
                    ELSE delivered_at
                END,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
              AND is_disputed = FALSE
              AND (
                (
                    $2::order_status = 'SHIPPED'::order_status
                    AND status IN ('PAID'::order_status, 'SHIPPED'::order_status)
                )
                OR
                (
                    $2::order_status = 'DELIVERED'::order_status
                    AND status IN ('SHIPPED'::order_status, 'DELIVERED'::order_status)
                )
              )
            "#,
        )
        .bind(order_id)
        .bind(target_status)
        .bind(target_shipping_status)
        .bind(tracking_number)
        .bind(should_set_delivered_at)
        .execute(&self.pool)
        .await
        .map_err(|error| OrderError::Database(error.into()))?;

        map_order_mutation_result(&self.pool, order_id, update.rows_affected()).await
    }

    async fn record_event(
        &self,
        order_id: OrderId,
        _event: &str,
        _metadata: Option<serde_json::Value>,
    ) -> Result<(), OrderError> {
        let update = sqlx::query(
            r#"
            UPDATE orders
            SET updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            "#,
        )
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(|error| OrderError::Database(error.into()))?;

        if update.rows_affected() == 0 {
            return Err(OrderError::NotFound);
        }
        Ok(())
    }
}

async fn map_order_mutation_result(
    pool: &PgPool,
    order_id: OrderId,
    affected_rows: u64,
) -> Result<(), OrderError> {
    if affected_rows > 0 {
        return Ok(());
    }

    let existing = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM orders
        WHERE id = $1
        "#,
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| OrderError::Database(error.into()))?;

    match existing {
        Some(_) => Err(OrderError::InvalidTransition),
        None => Err(OrderError::NotFound),
    }
}

fn map_row_to_order(row: PgRow) -> Result<Order, OrderError> {
    let id: Uuid = row.try_get("id").map_err(sqlx_error)?;
    let lot: String = row.try_get("title").map_err(sqlx_error)?;
    let buyer_id: Uuid = row.try_get("buyer_id").map_err(sqlx_error)?;
    let seller_id: Uuid = row.try_get("seller_id").map_err(sqlx_error)?;
    let final_price: i64 = row.try_get("final_price").map_err(sqlx_error)?;
    let raw_status: String = row.try_get("status_text").map_err(sqlx_error)?;
    let raw_shipping_status: Option<String> =
        row.try_get("shipping_status_text").map_err(sqlx_error)?;
    let carrier: Option<String> = row.try_get("carrier").map_err(sqlx_error)?;
    let tracking_number: Option<String> = row.try_get("tracking_number").map_err(sqlx_error)?;
    let is_disputed: bool = row.try_get("is_disputed").map_err(sqlx_error)?;
    let created_at: DateTime<Utc> = row.try_get("created_at").map_err(sqlx_error)?;
    let updated_at: DateTime<Utc> = row.try_get("updated_at").map_err(sqlx_error)?;

    let status = map_order_status(&raw_status, raw_shipping_status.as_deref(), is_disputed)?;
    let stage = map_order_stage(&raw_status, &status);
    let tags = build_tags(carrier, tracking_number, is_disputed);

    Ok(Order {
        id,
        lot,
        stage,
        status,
        buyer_id: buyer_id.to_string(),
        seller_id: seller_id.to_string(),
        total: format_currency(final_price),
        currency: "USD".to_string(),
        created_at: created_at.to_rfc3339(),
        updated_at: updated_at.to_rfc3339(),
        tags,
        last_activity: build_last_activity_label(&raw_status, is_disputed),
    })
}

fn map_order_status(
    raw_status: &str,
    shipping_status: Option<&str>,
    is_disputed: bool,
) -> Result<OrderStatus, OrderError> {
    if is_disputed || raw_status == "DISPUTED" {
        return Ok(OrderStatus::DisputeAlert);
    }

    let status = match raw_status {
        "PENDING_PAYMENT" => OrderStatus::AwaitingPayment,
        "PAID" => OrderStatus::AwaitingPayment,
        "SHIPPED" => OrderStatus::InTransit,
        "DELIVERED" => OrderStatus::NeedsConfirmation,
        "CONFIRMED" => OrderStatus::Delivered,
        "REFUNDED" | "CANCELLED" => OrderStatus::DisputeClosed,
        _ => {
            return Err(OrderError::Database(anyhow!(
                "unsupported order status from db: {raw_status}"
            )))
        }
    };

    if matches!(shipping_status, Some("SHIPPED")) {
        return Ok(OrderStatus::InTransit);
    }
    if matches!(shipping_status, Some("DELIVERED")) && raw_status != "CONFIRMED" {
        return Ok(OrderStatus::NeedsConfirmation);
    }

    Ok(status)
}

fn map_order_stage(raw_status: &str, status: &OrderStatus) -> OrderStage {
    match raw_status {
        "PENDING_PAYMENT" | "PAID" => OrderStage::Active,
        "SHIPPED" | "DELIVERED" | "DISPUTED" => OrderStage::Processing,
        "CONFIRMED" | "REFUNDED" => OrderStage::Completed,
        "CANCELLED" => OrderStage::Cancelled,
        _ => match status {
            OrderStatus::AwaitingPayment => OrderStage::Active,
            OrderStatus::InTransit | OrderStatus::NeedsConfirmation | OrderStatus::DisputeAlert => {
                OrderStage::Processing
            }
            OrderStatus::Delivered | OrderStatus::DisputeClosed => OrderStage::Completed,
        },
    }
}

fn build_tags(
    carrier: Option<String>,
    tracking_number: Option<String>,
    is_disputed: bool,
) -> Vec<String> {
    let mut tags = Vec::new();

    if let Some(carrier_name) = carrier
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        tags.push(format!("Courier: {}", carrier_name));
    }

    if let Some(tracking) = tracking_number
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        tags.push(format!("Tracking: {}", tracking));
    }

    if is_disputed {
        tags.push("Dispute Active".to_string());
    }

    tags
}

fn build_last_activity_label(raw_status: &str, is_disputed: bool) -> String {
    if is_disputed || raw_status == "DISPUTED" {
        return "Dispute opened by buyer".to_string();
    }

    match raw_status {
        "PENDING_PAYMENT" | "PAID" => "Awaiting payment settlement".to_string(),
        "SHIPPED" => "Seller updated shipment status".to_string(),
        "DELIVERED" => "Package delivered, waiting buyer confirmation".to_string(),
        "CONFIRMED" => "Buyer confirmed receipt".to_string(),
        "REFUNDED" | "CANCELLED" => "Order resolved".to_string(),
        _ => "Order updated".to_string(),
    }
}

fn format_currency(amount: i64) -> String {
    let is_negative = amount < 0;
    let mut digits = amount.abs().to_string().chars().rev().collect::<Vec<_>>();
    let mut grouped = String::new();
    for (index, digit) in digits.drain(..).enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    let mut formatted = grouped.chars().rev().collect::<String>();
    if is_negative {
        formatted = format!("-{}", formatted);
    }
    format!("${}", formatted)
}

fn sqlx_error(error: sqlx::Error) -> OrderError {
    OrderError::Database(error.into())
}
