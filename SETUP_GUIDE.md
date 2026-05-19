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

## Configuration

Buat file `.env`:

```env
APP_SERVER_HOST=0.0.0.0
APP_SERVER_PORT=8081
APP_DATABASE_URL=postgres://postgres:password@localhost:5432/bidmart_core
APP_AUTH_BASE_URL=http://localhost:8080
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
cargo run
```

## Tests

```bash
cargo test
```
