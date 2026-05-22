# Order and Notification Profiling Report

## Context

- Module: `order` and `notifications`
- Environment: Dummy Neon PostgreSQL, DB-backed test harness, no real auth service
- Date: 2026-05-22
- Baseline commit: `b3a160a`
- After commit: pending optimization
- Database size: synthetic profiling dataset seeded by the DB-backed profiler
- Test data notes: deterministic buyer, seller, order, listing, auction, and notification records
- Result file: `performance/results/db-backed-order-notification-profile-20260522-152947.json`

## Baseline Command

```powershell
$env:APP_DATABASE_URL = "postgresql://USER:PASSWORD@HOST/neondb?sslmode=require"
$env:ORDER_PROFILE_ITERATIONS = "50"
$env:ORDER_PROFILE_WARMUP = "5"
$env:ORDER_PROFILE_APDEX_MS = "500"
cargo test --lib profile_order_notifications_with_database -- --ignored --nocapture
```

## Baseline API Results

![alt text](<../images/Screenshot 2026-05-22 223624.png>)

## Baseline Database Findings

- Slowest API p95: `notifications_all` at `107.167 ms`.
- Highest API p99: `buyer_orders_all` at `359.990 ms`. This looks like an external database/network outlier because the SQL execution time is low.
- `buyer_order_list`: uses `idx_orders_buyer_id`, execution time `0.118 ms`, shared buffers hit `2`.
- `seller_order_list`: uses `idx_orders_seller_id`, execution time `0.919 ms`, shared buffers hit `1`, read `1`.
- `notification_unread_list`: uses `idx_notifications_user_id`, then filters `NOT is_read`, execution time `0.074 ms`, shared buffers hit `2`.
- No sequential scan was found in the captured baseline `EXPLAIN ANALYZE` output.
- Potential weak index: unread notifications currently rely on `idx_notifications_user_id` plus filter on `is_read`; a composite or partial index for unread notification listing may be useful if the real dataset grows.

## Optimization Candidate

- Candidate change: add a notification listing index that matches the unread listing query pattern.
- Possible SQL:

```sql
CREATE INDEX IF NOT EXISTS idx_notifications_user_unread_created
ON notifications(user_id, is_read, created_at DESC);
```

- Why this may help: `GET /notifications?unreadOnly=true` filters by `user_id` and `is_read`, then sorts by `created_at DESC`. The current baseline uses only `user_id`, so a larger dataset may require more filtering and sorting.
- Risk: low, because this is a database index and does not change the application flow.

## After API Results

Pending optimization and re-profiling.

| Endpoint | Count | Error Count | Avg ms | P50 ms | P95 ms | P99 ms | APDEX |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| buyer_orders_all | | | | | | | |
| buyer_orders_processing | | | | | | | |
| seller_orders_all | | | | | | | |
| buyer_order_detail | | | | | | | |
| seller_order_detail | | | | | | | |
| notifications_all | | | | | | | |
| notifications_unread | | | | | | | |
| notification_detail | | | | | | | |

## Before and After Summary

Pending optimization and re-profiling.

| Metric | Before | After | Improvement |
| --- | ---: | ---: | ---: |
| Worst endpoint p95 | 107.167 ms | | |
| Highest endpoint p99 | 359.990 ms | | |
| Average APDEX | 1.000 | | |
| Slowest SQL execution time | 0.919 ms | | |

## Screenshot Evidence

Recommended screenshots for the final report:

1. Terminal output showing the successful DB-backed profiling run.
2. The `Baseline API Results` table in this report.
3. The `Baseline Database Findings` section in this report.
4. After optimization, screenshot the updated `Before and After Summary` table.

## Conclusion

The baseline profiling run completed successfully with zero API errors and APDEX
`1.000` for all profiled order and notification endpoints. Current SQL execution
time is already low on the synthetic dataset, so the most defensible next step is
to apply a small database index optimization for notification listing, run the
same profiler again, and compare the before/after p95, p99, APDEX, and SQL
execution time.
