-- Order and notification database profiling.
--
-- Usage:
--   psql "$APP_DATABASE_URL" \
--     -v buyer_id="'<buyer-uuid>'" \
--     -v seller_id="'<seller-uuid>'" \
--     -v order_id="'<order-uuid>'" \
--     -v notification_id="'<notification-uuid>'" \
--     -f performance/sql/explain-order-notifications.sql

\echo '1) buyer order list'
EXPLAIN (ANALYZE, BUFFERS)
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
WHERE buyer_id = :buyer_id::uuid
ORDER BY updated_at DESC;

\echo '2) seller order list'
EXPLAIN (ANALYZE, BUFFERS)
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
WHERE seller_id = :seller_id::uuid
ORDER BY updated_at DESC;

\echo '3) order detail'
EXPLAIN (ANALYZE, BUFFERS)
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
WHERE id = :order_id::uuid;

\echo '4) notification list'
EXPLAIN (ANALYZE, BUFFERS)
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
WHERE user_id = :buyer_id::uuid
ORDER BY created_at DESC
LIMIT 20;

\echo '5) unread notification list'
EXPLAIN (ANALYZE, BUFFERS)
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
WHERE user_id = :buyer_id::uuid
  AND is_read = FALSE
ORDER BY created_at DESC
LIMIT 20;

\echo '6) notification detail'
EXPLAIN (ANALYZE, BUFFERS)
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
WHERE id = :notification_id::uuid;

\echo '7) existing indexes for order/notification tables'
SELECT
    schemaname,
    tablename,
    indexname,
    indexdef
FROM pg_indexes
WHERE tablename IN ('orders', 'notifications', 'disputes')
ORDER BY tablename, indexname;
