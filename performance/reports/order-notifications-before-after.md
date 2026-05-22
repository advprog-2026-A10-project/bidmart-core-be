# Order and Notification Profiling Report

## Context

- Module: `order` and `notifications`
- Environment: Dummy Neon PostgreSQL, DB-backed test harness, no real auth service
- Date: 2026-05-22
- Baseline report commit: this commit; see `git log --oneline`
- After commit: pending optimization
- Database size: synthetic profiling dataset seeded by the DB-backed profiler
- Test data notes: deterministic buyer, seller, order, listing, auction, and 2,000 notification records
- Result file: `performance/results/db-backed-order-notification-profile-20260522-161205.json`

## Baseline Command

```powershell
$env:APP_DATABASE_URL = "postgresql://USER:PASSWORD@HOST/neondb?sslmode=require"
$env:ORDER_PROFILE_ITERATIONS = "50"
$env:ORDER_PROFILE_WARMUP = "5"
$env:ORDER_PROFILE_APDEX_MS = "500"
cargo test --lib profile_order_notifications_with_database -- --ignored --nocapture
```

## Baseline API Results

| Endpoint | Count | Error Count | Avg ms | P50 ms | P95 ms | P99 ms | APDEX |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| buyer_orders_all | 50 | 0 | 87.319 | 78.853 | 140.089 | 194.349 | 1.000 |
| buyer_orders_processing | 50 | 0 | 70.605 | 68.294 | 84.897 | 88.827 | 1.000 |
| seller_orders_all | 50 | 0 | 71.758 | 70.844 | 92.310 | 94.390 | 1.000 |
| buyer_order_detail | 50 | 0 | 71.713 | 71.720 | 81.224 | 90.867 | 1.000 |
| seller_order_detail | 50 | 0 | 71.566 | 70.714 | 82.395 | 95.969 | 1.000 |
| notifications_all | 50 | 0 | 79.707 | 72.986 | 111.518 | 116.078 | 1.000 |
| notifications_unread | 50 | 0 | 79.001 | 73.427 | 109.053 | 132.045 | 1.000 |
| notification_detail | 50 | 0 | 70.711 | 69.712 | 81.422 | 95.817 | 1.000 |

## Baseline Database Findings

- Slowest API p95: `buyer_orders_all` at `140.089 ms`.
- Slowest notification API p95: `notifications_all` at `111.518 ms`.
- Highest API p99: `buyer_orders_all` at `194.349 ms`.
- `buyer_order_list`: uses `idx_orders_buyer_id`, execution time `0.064 ms`, shared buffers hit `2`.
- `seller_order_list`: uses `idx_orders_seller_id`, execution time `1.004 ms`, shared buffers hit `1`, read `1`.
- `notification_unread_list`: uses backward scan on `idx_notifications_created_at`, then filters by `user_id` and `NOT is_read`, execution time `0.051 ms`, shared buffers hit `3`.
- Sequential scans: none found in captured baseline `EXPLAIN ANALYZE`.
- Missing or weak indexes: notification unread listing does not use an index that directly matches `user_id`, `is_read`, and `created_at DESC`.

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
| Worst endpoint p95 | 140.089 ms | | |
| Worst notification endpoint p95 | 111.518 ms | | |
| Highest endpoint p99 | 194.349 ms | | |
| Average APDEX | 1.000 | | |
| Slowest SQL execution time | 1.004 ms | | |

## Screenshot Evidence

Recommended screenshots for the final report:

1. Terminal output showing the successful DB-backed profiling run.
2. The `Baseline API Results` table in this report.
3. The `Baseline Database Findings` section in this report.
4. After optimization, screenshot the updated `Before and After Summary` table.

## Conclusion

The 2,000-notification baseline profiling run completed successfully with zero
API errors and APDEX `1.000` for all profiled order and notification endpoints.
The main optimization opportunity is notification listing: the unread query is
currently planned from `idx_notifications_created_at` and then filtered by
`user_id` and `is_read`, so a composite notification listing index is the next
candidate to validate.
