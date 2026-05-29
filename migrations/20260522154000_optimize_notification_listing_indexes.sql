-- Optimize notification inbox listing queries.
--
-- The order/notification module commonly reads a user's latest notifications,
-- optionally filtered to unread only. These indexes match the WHERE clauses and
-- ORDER BY created_at DESC used by the repository.

CREATE INDEX IF NOT EXISTS idx_notifications_user_created_at
ON notifications(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_notifications_user_read_created_at
ON notifications(user_id, is_read, created_at DESC);
