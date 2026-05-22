# Order and Notification Profiling Report

## Context

- Module: `order` and `notifications`
- Environment:
- Date:
- Baseline commit:
- After commit:
- Database size:
- Test data notes:

## Commands

API profiling:

```powershell
powershell -ExecutionPolicy Bypass -File .\performance\scripts\profile-order-notifications.ps1 `
  -BaseUrl "http://127.0.0.1:8081/api/v1" `
  -Iterations 50 `
  -Warmup 5
```

Database profiling:

```powershell
psql "$env:APP_DATABASE_URL" `
  -v buyer_id="'<buyer-uuid>'" `
  -v seller_id="'<seller-uuid>'" `
  -v order_id="'<order-uuid>'" `
  -v notification_id="'<notification-uuid>'" `
  -f .\performance\sql\explain-order-notifications.sql
```

## Baseline API Results

### In-Memory Baseline

This section is optional and can be filled before a database is available. It
measures router/controller/use-case overhead with in-memory repositories.

Command:

```powershell
$env:ORDER_PROFILE_ITERATIONS = "200"
$env:ORDER_PROFILE_WARMUP = "20"
$env:ORDER_PROFILE_APDEX_MS = "10"
cargo test --lib profile_order_notifications_without_database -- --ignored --nocapture
```

| Endpoint | Count | Error Count | Avg ms | P50 ms | P95 ms | P99 ms | APDEX |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| buyer_orders_all | | | | | | | |
| buyer_orders_active | | | | | | | |
| buyer_orders_processing | | | | | | | |
| seller_orders_all | | | | | | | |
| buyer_order_detail | | | | | | | |
| seller_order_detail | | | | | | | |
| notifications_all | | | | | | | |
| notifications_unread | | | | | | | |
| notification_detail | | | | | | | |

### Runtime API Baseline

| Endpoint | Count | Error Count | Avg ms | P50 ms | P95 ms | P99 ms | APDEX |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| buyer_orders_all | | | | | | | |
| buyer_orders_active | | | | | | | |
| buyer_orders_processing | | | | | | | |
| seller_orders_all | | | | | | | |
| notifications_all | | | | | | | |
| notifications_unread | | | | | | | |

## Baseline Database Findings

Paste the important `EXPLAIN (ANALYZE, BUFFERS)` findings here.

- Slowest query:
- Sequential scans:
- Missing or weak indexes:
- Highest buffer usage:

## Optimization

- Change summary:
- Files changed:
- Why this should improve performance:

## After API Results

| Endpoint | Count | Error Count | Avg ms | P50 ms | P95 ms | P99 ms | APDEX |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| buyer_orders_all | | | | | | | |
| buyer_orders_active | | | | | | | |
| buyer_orders_processing | | | | | | | |
| seller_orders_all | | | | | | | |
| notifications_all | | | | | | | |
| notifications_unread | | | | | | | |

## Before and After Summary

| Metric | Before | After | Improvement |
| --- | ---: | ---: | ---: |
| Worst endpoint p95 | | | |
| Average APDEX | | | |
| Slowest SQL execution time | | | |

## Conclusion

Summarize whether the optimization materially improved order and notification
performance, and whether the result is enough to justify the change.
