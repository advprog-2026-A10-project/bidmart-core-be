# BidMart Core BE

Rust Axum backend untuk domain core BidMart (`catalog`, `bidding`, `wallet`, `order`).

## Status

Service aktif dan sudah terhubung dengan `bidmart-auth-be` untuk validasi sesi melalui `POST /auth/validate`.

## Docs

- Setup: [`SETUP_GUIDE.md`](./SETUP_GUIDE.md)
- Auth handoff: [`AUTH_HANDOFF.md`](./AUTH_HANDOFF.md)
- Catalog contract freeze (iterasi core-1): [`docs/CATALOG_ITER1_CONTRACT.md`](./docs/CATALOG_ITER1_CONTRACT.md)
- Wallet contract freeze (iterasi core-2): [`docs/WALLET_ITER1_CONTRACT.md`](./docs/WALLET_ITER1_CONTRACT.md)
- Bidding contract freeze (iterasi core-3): [`docs/BIDDING_ITER1_CONTRACT.md`](./docs/BIDDING_ITER1_CONTRACT.md)
- Staging logical replication runbook (auth+core): [`docs/STAGING_LOGICAL_REPLICATION_RUNBOOK.md`](./docs/STAGING_LOGICAL_REPLICATION_RUNBOOK.md)

## Quick Start

```bash
cargo build
cargo run --bin migrate
cargo run
```

For containerized deployment, run the migration step as a one-shot task before booting the API service.
`APP_AUTO_MIGRATE_ON_STARTUP=true` tersedia sebagai fallback, namun default yang direkomendasikan adalah `false`.

## Test

```bash
cargo test
```
