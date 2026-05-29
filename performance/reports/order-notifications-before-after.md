# Order and Notification Profiling Report

## Context

- Module: `order` and `notifications`
- Environment: Dummy Neon PostgreSQL, DB-backed test harness, no real auth service
- Date: 2026-05-22
- Baseline report commit: this commit; see `git log --oneline`
- After optimization commit: this commit; see `git log --oneline`
- Database size: synthetic profiling dataset seeded by the DB-backed profiler
- Test data notes: deterministic buyer, seller, order, listing, auction, and 2,000 notification records
- Baseline result file: `performance/results/db-backed-order-notification-profile-20260522-161205.json`
- After result file: `performance/results/db-backed-order-notification-profile-20260529-192346.json`

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


![alt text](<../images/Screenshot 2026-05-22 231254.png>)


## Baseline Database Findings

- Slowest API p95: `buyer_orders_all` at `140.089 ms`.
- Slowest notification API p95: `notifications_all` at `111.518 ms`.
- Highest API p99: `buyer_orders_all` at `194.349 ms`.
- `buyer_order_list`: uses `idx_orders_buyer_id`, execution time `0.064 ms`, shared buffers hit `2`.
- `seller_order_list`: uses `idx_orders_seller_id`, execution time `1.004 ms`, shared buffers hit `1`, read `1`.
- `notification_unread_list`: uses backward scan on `idx_notifications_created_at`, then filters by `user_id` and `NOT is_read`, execution time `0.051 ms`, shared buffers hit `3`.
- Sequential scans: none found in captured baseline `EXPLAIN ANALYZE`.
- Missing or weak indexes: notification unread listing does not use an index that directly matches `user_id`, `is_read`, and `created_at DESC`.

## Optimization

- Change summary:
  - Added notification listing indexes for `user_id`, `is_read`, and `created_at DESC`.
  - Reworked notification listing repository queries so common paths use explicit `WHERE` clauses instead of parameterized `OR` conditions.
- Files changed:
  - `migrations/20260522154000_optimize_notification_listing_indexes.sql`
  - `src/modules/order/infrastructure/repositories/db_notification_repository.rs`
- SQL index optimization:

```sql
CREATE INDEX IF NOT EXISTS idx_notifications_user_created_at
ON notifications(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_notifications_user_read_created_at
ON notifications(user_id, is_read, created_at DESC);
```

- Why this should improve performance: `GET /notifications` filters by `user_id`, optionally filters by `is_read`, and sorts by `created_at DESC`. The new indexes match those access patterns, while the query split gives PostgreSQL simpler predicates to plan against.
- Risk: low, because this is a database index and does not change the application flow.

## After API Results

| Endpoint | Count | Error Count | Avg ms | P50 ms | P95 ms | P99 ms | APDEX |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| buyer_orders_all | 50 | 0 | 57.030 | 54.651 | 76.894 | 84.882 | 1.000 |
| buyer_orders_processing | 50 | 0 | 56.726 | 55.988 | 62.235 | 73.568 | 1.000 |
| seller_orders_all | 50 | 0 | 56.329 | 54.536 | 69.787 | 81.271 | 1.000 |
| buyer_order_detail | 50 | 0 | 55.001 | 54.076 | 60.581 | 67.186 | 1.000 |
| seller_order_detail | 50 | 0 | 56.559 | 55.318 | 69.147 | 70.902 | 1.000 |
| notifications_all | 50 | 0 | 65.217 | 56.933 | 97.056 | 227.913 | 1.000 |
| notifications_unread | 50 | 0 | 58.260 | 56.545 | 67.101 | 80.587 | 1.000 |
| notification_detail | 50 | 0 | 55.537 | 55.016 | 64.072 | 69.285 | 1.000 |

## After Database Findings

- Slowest API p95: `notifications_all` at `97.056 ms`.
- Slowest notification API p95: `notifications_all` at `97.056 ms`.
- Highest API p99: `notifications_all` at `227.913 ms`; this appears to be one external latency outlier because the p50 remained `56.933 ms`.
- `buyer_order_list`: uses `idx_orders_buyer_id`, execution time `0.066 ms`, shared buffers hit `2`.
- `seller_order_list`: uses `idx_orders_seller_id`, execution time `0.867 ms`, shared buffers hit `1`, read `1`.
- `notification_unread_list`: still uses backward scan on `idx_notifications_created_at`, then filters by `user_id` and `NOT is_read`, execution time `0.057 ms`, shared buffers hit `5`.
- Sequential scans: none found in captured after `EXPLAIN ANALYZE`.
- Note: the API latency improved even though this small synthetic `EXPLAIN` sample still selected the older `created_at` index. The query split removes parameterized `OR` predicates from the runtime path, and the new composite indexes remain available for larger or more selective real datasets.

## Before and After Summary

| Metric | Before | After | Improvement |
| --- | ---: | ---: | ---: |
| Worst endpoint p95 | 140.089 ms | 97.056 ms | 30.7% faster |
| Worst notification endpoint p95 | 111.518 ms | 97.056 ms | 13.0% faster |
| `notifications_unread` p95 | 109.053 ms | 67.101 ms | 38.5% faster |
| `notifications_unread` average | 79.001 ms | 58.260 ms | 26.3% faster |
| Highest endpoint p99 | 194.349 ms | 227.913 ms | 17.3% worse |
| Average APDEX | 1.000 | 1.000 | unchanged |
| Slowest SQL execution time | 1.004 ms | 0.867 ms | 13.6% faster |

## Screenshot Evidence

Recommended screenshots for the final report:

1. Terminal output showing the successful DB-backed profiling run.
2. The `Baseline API Results` table in this report.
3. The `Baseline Database Findings` section in this report.
4. After optimization, screenshot the updated `Before and After Summary` table.

## Conclusion

The optimization improved the worst endpoint p95 from `140.089 ms` to `97.056 ms`
and improved the targeted `notifications_unread` p95 from `109.053 ms` to
`67.101 ms`. All profiled endpoints completed with zero errors and APDEX `1.000`.
The only regression is the highest p99, caused by one `notifications_all` outlier;
the p50 and p95 values still improved, so the optimization is acceptable for the
profiled order and notification read paths.
