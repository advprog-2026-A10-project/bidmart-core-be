# Order and Notification Performance Profiling

This folder contains profiling helpers for the `order` and `notifications`
module. The helpers are intentionally kept outside production code so they do
not change the application flow owned by other team members.

## Scope

The default profiling suite is read-only:

- `GET /orders`
- `GET /orders?stage=active`
- `GET /orders?stage=processing`
- `GET /seller/orders`
- `GET /notifications?limit=20`
- `GET /notifications?limit=20&unreadOnly=true`
- `GET /orders/:orderId` when `-OrderId` is provided
- `GET /seller/orders/:orderId` when `-OrderId` is provided
- `GET /notifications/:notificationId` when `-NotificationId` is provided

Mutation endpoints such as shipping update, confirm order, dispute creation,
and mark-as-read should be profiled in an isolated staging dataset because they
change application state.

## API Latency Profiling

Start the core backend in another terminal, then run:

```powershell
cd C:\rustgroup\bidmart-core-be
powershell -ExecutionPolicy Bypass -File .\performance\scripts\profile-order-notifications.ps1 `
  -BaseUrl "http://127.0.0.1:8081/api/v1" `
  -Iterations 50 `
  -Warmup 5
```

If auth is enabled, pass a token or cookie from a valid session:

```powershell
powershell -ExecutionPolicy Bypass -File .\performance\scripts\profile-order-notifications.ps1 `
  -BaseUrl "http://127.0.0.1:8081/api/v1" `
  -BearerToken "<access-token>" `
  -Iterations 50 `
  -Warmup 5
```

To include detail endpoints:

```powershell
powershell -ExecutionPolicy Bypass -File .\performance\scripts\profile-order-notifications.ps1 `
  -BaseUrl "http://127.0.0.1:8081/api/v1" `
  -OrderId "<order-uuid>" `
  -NotificationId "<notification-uuid>"
```

The script writes:

- `performance/results/order-notification-profile-*.csv`
- `performance/results/order-notification-profile-*.json`

The JSON summary includes count, error count, average latency, p50, p95, p99,
and APDEX. The default APDEX satisfied threshold is 500 ms.

## In-Memory Profiling Without Database

If the database or auth service is not available, run the test-only in-memory
profile first:

```powershell
cd C:\rustgroup\bidmart-core-be
$env:ORDER_PROFILE_ITERATIONS = "200"
$env:ORDER_PROFILE_WARMUP = "20"
$env:ORDER_PROFILE_APDEX_MS = "10"
cargo test --lib profile_order_notifications_without_database -- --ignored --nocapture
```

This profile uses the order module's in-memory test repositories and exercises
the Axum router/controller/use-case path without SQL. It is useful for an early
non-DB baseline, but it does not measure database query performance.

The output is written to:

- `performance/results/in-memory-order-notification-profile-*.json`

## DB-Backed Profiling Without Real Auth

For a disposable profiling database such as a dummy Neon project, run the
DB-backed manual profiler. It seeds a deterministic synthetic listing, auction,
order, and 2,000 notifications into the database, then exercises the real SQLx
order and notification repositories through the Axum router. Auth is disabled
only in this manual test harness; production config is not changed.

```powershell
cd C:\rustgroup\bidmart-core-be
$env:APP_DATABASE_URL = "postgresql://USER:PASSWORD@HOST/neondb?sslmode=require"
$env:ORDER_PROFILE_ITERATIONS = "50"
$env:ORDER_PROFILE_WARMUP = "5"
$env:ORDER_PROFILE_APDEX_MS = "500"
cargo test --lib profile_order_notifications_with_database -- --ignored --nocapture
```

The output is written to:

- `performance/results/db-backed-order-notification-profile-*.json`

## Database Profiling

Run the SQL script against a database with representative data:

```powershell
psql "$env:APP_DATABASE_URL" `
  -v buyer_id="'<buyer-uuid>'" `
  -v seller_id="'<seller-uuid>'" `
  -v order_id="'<order-uuid>'" `
  -v notification_id="'<notification-uuid>'" `
  -f .\performance\sql\explain-order-notifications.sql
```

Save the output in the report template under `performance/reports/`.

## Before and After Workflow

1. Check out the baseline commit.
2. Run API latency profiling and database `EXPLAIN ANALYZE`.
3. Save results in a copied report template.
4. Apply a small optimization, for example an index or query improvement.
5. Run the same profiling commands again.
6. Compare p95 latency, APDEX, and `EXPLAIN ANALYZE` execution time.

For the final rubric, the strongest evidence is a before/after commit pair with
profiling results that show measurable improvement.
