# Clean Architecture Axum Project Setup Guide

This document provides instructions for agents to replicate the `bidmart-core-be` project structure.

## Project Overview

- **Project Name**: bidmart-core-be
- **Type**: Rust Axum REST API
- **Architecture**: Modular Clean Architecture
- **Database**: PostgreSQL with SQLx (separate from bidmart-auth-be)
- **Authentication**: Calls bidmart-auth-be microservice

## Directory Structure

```
<project-name> /
├── Cargo.toml
├── build.rs
├── .env.example
├── migrations/                  # SQLx migrations
│   └── YYYYMMDDHHMMSS_description.sql
└── src/
    ├── main.rs
    ├── lib.rs
    ├── infrastructure/           # Global Technical & Configuration
    │   ├── config/             # Environment Variables
    │   ├── database/           # DB Connection Pool
    │   ├── logger/             # Logger Implementation
    │   └── filters/            # Global Exception Filters
    │
    ├── modules/                 # Feature Modules (Vertical Slices)
    │   └── catalog/           # Catalog Module (placeholder)
    │       ├── application/   # Use Cases & DTOs
    │       │   ├── dto/
    │       │   └── use_cases/
    │       ├── domain/        # Entities, Errors, Repository Interfaces
    │       │   ├── entities/
    │       │   ├── errors/
    │       │   └── traits/
    │       └── infrastructure/ # Controllers, Repositories & Services
    │           ├── controllers/
    │           ├── repositories/
    │           ├── services/
    │           └── middleware/
    │
    └── shared/                 # Cross-cutting Components
        └── domain/            # Result Pattern, etc.
```

## Configuration

### Environment Variables

Create `.env` file:

```env
APP_SERVER_HOST=0.0.0.0
APP_SERVER_PORT=8081
APP_DATABASE_URL=postgres://postgres:password@localhost:5432/bidmart_core
APP_AUTH_BASE_URL=http://localhost:8080
```

### Key Differences from bidmart-auth-be

1. **Port**: 8081 (instead of 8080)
2. **Database**: Separate database (`bidmart_core`) 
3. **Auth**: Calls bidmart-auth-be microservice instead of JWT internally
4. **Module**: Placeholder catalog module (to be implemented later)

## API Endpoints

- `GET /health` - Health check
- `POST /api/v1/catalog/categories` - Create category
- `GET /api/v1/catalog/categories` - List categories
- `PUT /api/v1/catalog/categories/:id` - Update category

## Running the Project

```bash
cargo build
cargo run
```

## Running Tests

```bash
cargo test
```
