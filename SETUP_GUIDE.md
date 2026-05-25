# BidMart Core BE Setup Guide

Dokumen ini adalah panduan setup `bidmart-core-be` sesuai kondisi aktual codebase.

## Project Overview

- **Project Name**: `bidmart-core-be`
- **Type**: Rust Axum REST API
- **Architecture**: Modular Clean Architecture
- **Database**: PostgreSQL (`sqlx`)
- **Auth Integration**: Validasi sesi via `bidmart-auth-be` endpoint `POST /auth/validate`

## Active Modules

- `catalog` — listings, kategori, dan internal listing lifecycle.
- `bidding` — auction, bids, proxy bidding, finalizer worker.
- `wallet` — saldo, topup, withdraw, transactions, internal hold/release/payment.
- `order` — order lifecycle dan notifications/event endpoint internal.

## Important Docs

- Auth handoff: [`AUTH_HANDOFF.md`](./AUTH_HANDOFF.md)
- Catalog contract freeze iterasi core-1: [`docs/CATALOG_ITER1_CONTRACT.md`](./docs/CATALOG_ITER1_CONTRACT.md)
- Bidding contract freeze iterasi core-3: [`docs/BIDDING_ITER1_CONTRACT.md`](./docs/BIDDING_ITER1_CONTRACT.md)
- Wallet contract freeze iterasi core-2: [`docs/WALLET_ITER1_CONTRACT.md`](./docs/WALLET_ITER1_CONTRACT.md)
- Staging logical replication runbook: [`docs/STAGING_LOGICAL_REPLICATION_RUNBOOK.md`](./docs/STAGING_LOGICAL_REPLICATION_RUNBOOK.md)

## Directory Structure (top-level)

```text
bidmart-core-be/
├── Cargo.toml
├── build.rs
├── Dockerfile
├── .env.example
├── migrations/                 # SQLx migrations
├── docs/
├── tests/                      # Crate-level integration tests (workspace tests)
└── src/
    ├── main.rs
    ├── lib.rs
    ├── bin/
    │   └── migrate.rs          # One-shot migration binary
    ├── infrastructure/         # Global technical & configuration
    │   ├── auth/               # Auth client (validate session via auth-be)
    │   ├── config/             # Environment variables
    │   ├── database/           # DB connection pool
    │   ├── logger/             # Tracing setup
    │   └── filters/            # Global error handlers
    ├── modules/                # Feature modules (vertical slices)
    │   ├── catalog/
    │   ├── bidding/
    │   ├── wallet/
    │   └── order/
    └── shared/                 # Cross-cutting components
        └── domain/
```

## Per-Module Clean Architecture

Setiap modul di `src/modules/<module>/` mengikuti pola Clean Architecture tiga-layer yang konsisten.
Struktur kanonis (semua sub-direktori opsional kecuali `domain/` dan `application/`):

```text
modules/<module>/
├── application/
│   ├── dto/                    # Request/response DTOs (+ validator)
│   └── use_cases/              # Application use cases
│       └── tests/              # (opsional) Unit tests untuk use cases
│           ├── mod.rs
│           ├── support.rs      # Shared test fixtures / fakes
│           └── <use_case>.rs   # Satu file per use case yang ditest
├── domain/
│   ├── entities/               # Domain entities (struct + behavior)
│   ├── errors/                 # Domain-level error enums
│   └── traits/                 # Repository / port interfaces
├── infrastructure/
│   ├── controllers/            # Axum handlers / route glue
│   │   └── tests.rs            # (opsional) Unit tests yang butuh akses item private
│   ├── repositories/           # Implementasi repository (Postgres, dsb.)
│   ├── services/               # (opsional) Implementasi domain services
│   ├── middleware/             # (opsional) Tower middleware modul-spesifik
│   └── tests/                  # (opsional) Contract / integration-style tests
│       ├── mod.rs
│       └── <area>.rs
└── mod.rs
```

Penerapan aktual contoh `catalog`:

```text
src/modules/catalog/
├── application/
│   ├── dto/
│   │   ├── buyer_listing_dto.rs
│   │   ├── category_dto.rs
│   │   └── listing_dto.rs
│   └── use_cases/
│       ├── buyer_listing_use_cases.rs
│       ├── category_use_cases.rs
│       └── listing_use_cases.rs
├── domain/
│   ├── entities/
│   ├── errors/
│   └── traits/
└── infrastructure/
    ├── controllers/
    │   ├── buyer_controller.rs
    │   ├── category_controller.rs
    │   ├── internal_controller.rs
    │   └── seller_controller.rs
    ├── middleware/
    ├── repositories/
    └── services/
```

Modul `order` menggunakan subtree `infrastructure/tests/` untuk in-memory repositories dan
contract-style tests; modul `bidding` menggunakan sibling `infrastructure/controllers/tests.rs`
untuk test yang butuh akses item private. Lihat bagian *Test Placement Conventions* di bawah.

## Test Placement Conventions

Production code dan test code dipisahkan secara konsisten. Pakai pola berikut:

1. **`src/modules/<m>/application/use_cases/tests/`**
   - Unit tests untuk use case (logika aplikasi murni).
   - Satu file per use case, plus `support.rs` untuk fixtures / in-memory fakes.
   - Didaftarkan di `use_cases/mod.rs` dengan `#[cfg(test)] pub(crate) mod tests;`.

2. **`src/modules/<m>/infrastructure/tests/`**
   - Contract / integration-style tests pada level modul (mis. HTTP contract via `tower::ServiceExt`).
   - Boleh berisi in-memory repositories, support helpers, dan beberapa file test.
   - Contoh aktif: `src/modules/order/infrastructure/tests/`.

3. **`src/modules/<m>/infrastructure/controllers/tests.rs`** *(sibling file)*
   - Hanya digunakan ketika test perlu akses langsung ke item private di file implementasi controller.
   - Pakai pola ini sebagai pengecualian, bukan default.
   - Contoh aktif: `src/modules/bidding/infrastructure/controllers/tests.rs`.

4. **`tests/` di root project**
   - Crate-level integration tests (entity tests, service-level tests, dll).

Hindari menempatkan blok `#[cfg(test)] mod tests { ... }` panjang langsung di file implementasi utama —
selalu pindahkan ke subtree `tests/` atau sibling `tests.rs` sesuai konvensi di atas.

Status coverage internal test per modul saat ini:

- `order`: punya subtree `infrastructure/tests/` (in-memory repos + contracts + controllers tests).
- `bidding`: punya sibling `infrastructure/controllers/tests.rs`.
- `catalog` dan `wallet`: belum punya subtree test internal khusus di bawah `src/modules/*`;
  test coverage utamanya masih datang dari level crate (root `tests/`).

## Configuration

Buat file `.env`:

```env
APP_SERVER_HOST=0.0.0.0
APP_SERVER_PORT=8081
APP_DATABASE_URL=postgres://postgres:password@localhost:5432/bidmart_core
APP_AUTH_BASE_URL=http://localhost:8080
APP_AUTO_MIGRATE_ON_STARTUP=false
APP_BIDDING_FINALIZER_INTERVAL_SECS=5
APP_BIDDING_FINALIZER_BATCH_SIZE=25
APP_STORAGE_PROVIDER=minio
APP_STORAGE_ENDPOINT=http://localhost:9000
APP_STORAGE_BUCKET=bidmart-listing-images
APP_STORAGE_REGION=us-east-1
APP_STORAGE_ACCESS_KEY=minioadmin
APP_STORAGE_SECRET_KEY=minioadmin123
APP_STORAGE_FORCE_PATH_STYLE=true
APP_STORAGE_PUBLIC_BASE_URL=http://localhost:9000/bidmart-listing-images
```

## Runtime Endpoints

Semua endpoint aplikasi berada pada prefix `/api/v1` (Wallet juga menyediakan compat alias `/api/core/v1`).

### Catalog (public + seller)

- Buyer:
  - `GET /api/v1/catalog`
  - `GET /api/v1/c/*category_path`
  - `GET /api/v1/listings/:id`
- Seller (auth required):
  - `POST /api/v1/seller/listings/uploads/presign` (direct upload URL for MinIO/S3)
  - `GET /api/v1/seller/listings`
  - `POST /api/v1/seller/listings`
  - `GET /api/v1/seller/listings/:id`
  - `PATCH /api/v1/seller/listings/:id`
  - `DELETE /api/v1/seller/listings/:id`
  - `POST /api/v1/seller/listings/:id/publish`

### Categories

- `GET /api/v1/categories`
- `POST /api/v1/categories`
- `GET /api/v1/categories/:id`
- `PATCH /api/v1/categories/:id`
- `DELETE /api/v1/categories/:id`
- `GET /api/v1/categories/slug/:slug`

### Bidding

- `GET /api/v1/auctions/:auction_id`
- `GET /api/v1/auctions/:auction_id/history`
- `POST /api/v1/auctions/:auction_id/bids` — menerima optional `maxAmount` untuk aktifkan/update proxy bidding.
- `GET /api/v1/auctions/:auction_id/proxy`
- `PUT /api/v1/auctions/:auction_id/proxy`
- `DELETE /api/v1/auctions/:auction_id/proxy`
- `POST /api/v1/auctions/:auction_id/finalize?force=true|false`
- `GET /api/v1/me/bids?status=winning|outbid|won|lost`
- `GET /api/v1/me/bids/:auction_id`
- Realtime FE transport (primary) menggunakan WebSocket gateway eksternal, default lokal `ws://localhost:8080`.
- Project gateway lokal tersedia di root workspace: `../bidmart-bidding-ws`.
- Background worker otomatis finalisasi auction yang melewati `ends_at`; interval/batch diatur lewat
  `APP_BIDDING_FINALIZER_INTERVAL_SECS` dan `APP_BIDDING_FINALIZER_BATCH_SIZE`.

### Wallet

- Primary prefix: `/api/v1`
- Compatibility alias: `/api/core/v1`
- Public:
  - `GET /wallet`
  - `POST /wallet/topup`
  - `POST /wallet/withdraw`
  - `GET /wallet/transactions`
  - `GET /wallet/transactions/:transactionId`
- Internal:
  - `POST /internal/wallet/holds`
  - `POST /internal/wallet/release`
  - `POST /internal/wallet/payment`

### Order

Order masih menggunakan endpoint root saat ini; kontrak detail akan dibekukan pada iterasi modul order.

## Running

```bash
cargo build
cargo run --bin migrate
cargo run --bin db_reset
cargo run --bin db_seed
cargo run
```

### Optional: Run Local MinIO (from workspace root)

```bash
cd /home/kims/adpro
docker compose --env-file .env.minio -f docker-compose.minio.yml up -d
```

MinIO local endpoints:

- API: `http://localhost:9000`
- Console: `http://localhost:9001`
- Bucket initialized automatically: `bidmart-listing-images`

### Deployment Migration Step (Recommended)

Gunakan step migrasi eksplisit untuk deployment (terutama di Docker Compose), lalu baru start API:

```bash
# one-shot migration task
cargo run --bin migrate

# start API service
cargo run
```

Contoh dependency di Compose:

```yaml
services:
  core-be-migrate:
    image: ghcr.io/<org>/bidmart-core-be:<tag>
    command: ["/usr/local/bin/migrate"]
    environment:
      APP_DATABASE_URL: ${CORE_DATABASE_URL}
      APP_AUTH_BASE_URL: ${AUTH_BASE_URL}

  core-be:
    image: ghcr.io/<org>/bidmart-core-be:<tag>
    depends_on:
      core-be-migrate:
        condition: service_completed_successfully
```

`APP_AUTO_MIGRATE_ON_STARTUP=true` tersedia sebagai fallback, tetapi untuk staging/production
tetap disarankan `false` dan menggunakan migration job terpisah.

## Tests

```bash
cargo test
```

Pola yang dipakai di codebase saat ini:

- `order`:
  - module-local test support dan contract-style tests di `src/modules/order/infrastructure/tests/`.
- `bidding`:
  - helper/controller unit tests dipisah ke `src/modules/bidding/infrastructure/controllers/tests.rs`.
- `catalog` dan `wallet`:
  - belum punya subtree test internal khusus di bawah `src/modules/*`; test coverage utamanya masih
    datang dari level crate (root `tests/`).
