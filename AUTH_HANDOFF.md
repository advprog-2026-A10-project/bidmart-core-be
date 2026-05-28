# Core BE Auth Handoff (Iteration 1)

This document explains how `bidmart-core-be` now validates user sessions through `bidmart-auth-be`.

## 1) Upstream Contract

- Endpoint: `POST {APP_AUTH_BASE_URL}/auth/validate`
- Auth sources accepted upstream:
  - `Authorization: Bearer <token>`
  - `Cookie: auth_session=<cookie-value>`
- Expected success payload:

```json
{
  "userId": "uuid",
  "name": "Alice Johnson",
  "email": "alice@example.com",
  "emailVerified": true,
  "mfaSatisfied": true,
  "sessionExpiry": "2026-01-01T01:00:00+00:00"
}
```

## 2) What Changed in Core

- Catalog seller middleware no longer reads `x-debug-user-*` headers.
- Wallet user auth extractor no longer returns hardcoded test UUID.
- Bidding user endpoints now validate session through the same auth handoff path.
- Order and notification endpoints now use auth-backed runtime identity instead of placeholder headers.
- All of the above call `POST /auth/validate` via shared helper:
  - `src/infrastructure/auth/mod.rs`

## 3) Required Env

`APP_AUTH_BASE_URL` must point to the running auth service, for example:

```env
APP_AUTH_BASE_URL=http://localhost:8080
```

## 4) Migration Note

If any client/test still depends on debug header auth bypass (`x-debug-user-id`, `x-debug-user-name`), migrate them to provide a real bearer token (or valid cookie) issued by `bidmart-auth-be`.
