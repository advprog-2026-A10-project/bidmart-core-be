# BidMart Core BE Setup Guide

Dokumen ini adalah panduan setup `bidmart-core-be` sesuai kondisi aktual codebase.

## Project Overview

- **Project Name**: `bidmart-core-be`
- **Type**: Rust Axum REST API
- **Architecture**: Modular Clean Architecture
- **Database**: PostgreSQL (`sqlx`)
- **Auth Integration**: Validasi sesi via `bidmart-auth-be` endpoint `POST /auth/validate`

## Active Modules

- `catalog`
- `bidding`
- `wallet`
- `order` (termasuk notifications/event endpoint internal modul)

## Important Docs

- Auth handoff: [`AUTH_HANDOFF.md`](./AUTH_HANDOFF.md)
- Catalog contract freeze iterasi core-1: [`docs/CATALOG_ITER1_CONTRACT.md`](./docs/CATALOG_ITER1_CONTRACT.md)
- Bidding contract freeze iterasi core-3: [`docs/BIDDING_ITER1_CONTRACT.md`](./docs/BIDDING_ITER1_CONTRACT.md)
- Wallet contract freeze iterasi core-2: [`docs/WALLET_ITER1_CONTRACT.md`](./docs/WALLET_ITER1_CONTRACT.md)
- Staging logical replication runbook: [`docs/STAGING_LOGICAL_REPLICATION_RUNBOOK.md`](./docs/STAGING_LOGICAL_REPLICATION_RUNBOOK.md)

## Directory Structure (ringkas)

```text
bidmart-core-be/
├── Cargo.toml
├── .env.example
├── migrations/
└── src/
    ├── main.rs
    ├── infrastructure/
    │   ├── auth/
    │   ├── config/
    │   ├── database/
    │   ├── filters/
    │   └── logger/
    ├── modules/
    │   ├── catalog/
    │   ├── bidding/
    │   ├── wallet/
    │   └── order/
    └── shared/
```

Catatan struktur test:

- Production code dan test code dipisahkan di level modul.
- Gunakan subtree `tests/` untuk test/helper yang cukup besar, misalnya `src/modules/order/infrastructure/tests/`.
- Gunakan sibling `tests.rs` hanya bila test perlu akses langsung ke item private dalam file implementasi, misalnya `src/modules/bidding/infrastructure/controllers/tests.rs`.
- Hindari menaruh blok `#[cfg(test)] mod tests { ... }` panjang langsung di file implementasi utama.

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
```

## Runtime Endpoints (high-level)

Semua endpoint aplikasi berada pada prefix `/api/v1`.

### Catalog (public + seller)

- Buyer:
  - `GET /api/v1/catalog`
  - `GET /api/v1/c/*category_path`
  - `GET /api/v1/listings/:id`
- Seller (auth required):
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
- `POST /api/v1/auctions/:auction_id/bids`
- `POST /api/v1/auctions/:auction_id/bids` menerima optional `maxAmount` untuk aktifkan/update proxy bidding.
- `GET /api/v1/auctions/:auction_id/proxy`
- `PUT /api/v1/auctions/:auction_id/proxy`
- `DELETE /api/v1/auctions/:auction_id/proxy`
- `POST /api/v1/auctions/:auction_id/finalize?force=true|false`
- `GET /api/v1/me/bids?status=winning|outbid|won|lost`
- `GET /api/v1/me/bids/:auction_id`
- Realtime FE transport (primary) menggunakan WebSocket gateway eksternal, default lokal `ws://localhost:8080`.
- Project gateway lokal tersedia di root workspace:
  - `../bidmart-bidding-ws`
- Background worker:
  - otomatis finalisasi auction yang melewati `ends_at`
  - interval/batch bisa diatur lewat `APP_BIDDING_FINALIZER_INTERVAL_SECS` dan `APP_BIDDING_FINALIZER_BATCH_SIZE`

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
cargo run
```

## Deployment Migration Step (Recommended)

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

`APP_AUTO_MIGRATE_ON_STARTUP=true` tersedia sebagai fallback, tetapi untuk staging/production tetap disarankan `false` dan menggunakan migration job terpisah.

## Tests

```bash
cargo test
```

Pola yang dipakai di codebase saat ini:

- `order`:
  - module-local test support dan contract-style tests ada di `src/modules/order/infrastructure/tests/`
- `bidding`:
  - helper/controller unit tests dipisah ke `src/modules/bidding/infrastructure/controllers/tests.rs`
- `catalog` dan `wallet`:
  - saat ini belum punya subtree test internal khusus di bawah `src/modules/*`; test coverage utamanya masih datang dari level crate/workspace
