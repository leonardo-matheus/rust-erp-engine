# TechFix ERP

> Full-stack ERP built with **Rust**, **WebAssembly**, and **gRPC** — powered by **Sled** (lock-free B+Tree), sub-millisecond reads, PCI-grade security.

---

## Architecture

```
┌───────────────────────────────────────────────────────────────┐
│                       Browser (WASM)                          │
│  ┌───────────┐  ┌───────────┐  ┌──────────┐  ┌───────────┐  │
│  │ Dashboard │  │ Products  │  │  Sales   │  │   Stock   │  │
│  └─────┬─────┘  └─────┬─────┘  └────┬─────┘  └─────┬─────┘  │
│        └───────────────┴─────────────┴───────────────┘        │
│                         │ wasm-bindgen                        │
│               ┌─────────┴──────────┐                          │
│               │  Rust WASM Core    │                          │
│               └────────────────────┘                          │
└───────────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────────┐
│                    gRPC Backend (Rust)                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐ │
│  │  tonic   │  │  Argon2  │  │ AES-GCM  │  │    Sled      │ │
│  │  gRPC    │  │  Auth    │  │ Encrypt  │  │  B+Tree      │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └──────┬───────┘ │
│       └──────────────┴─────────────┴───────────────┘         │
│               ┌────────────────────┐                          │
│               │ Audit Log + RBAC   │                          │
│               └────────────────────┘                          │
└───────────────────────────────────────────────────────────────┘
```

## Performance Benchmarks

### gRPC API (Sled storage, release build, 50 concurrent connections)

| Endpoint | Avg | p50 | p90 | p99 | Throughput |
|----------|:---:|:---:|:---:|:---:|:----------:|
| **Health Check** | 1.44 ms | 1.25 ms | 2.62 ms | 4.52 ms | **19,296 req/s** |
| **List Sales** | 1.40 ms | 1.26 ms | 2.31 ms | 3.81 ms | **19,924 req/s** |
| **Stock Movements** | 1.48 ms | 1.32 ms | 2.50 ms | 4.26 ms | **19,726 req/s** |
| **Dashboard Stats** | 1.76 ms | 1.65 ms | 2.79 ms | 4.47 ms | **18,072 req/s** |
| **List Products** | 2.57 ms | 2.34 ms | 4.29 ms | 6.92 ms | **13,958 req/s** |
| **Login (Argon2id)** | 108.06 ms | 107.45 ms | 139.72 ms | 165.82 ms | **182 req/s** |

### Concurrency Stress Tests

| Test | Connections | Requests | Avg | p99 | Throughput | Errors |
|------|:----------:|:--------:|:---:|:---:|:----------:|:------:|
| **Dashboard** | 200 | 10,000 | 6.59 ms | 12.39 ms | **21,989 req/s** | 0 |
| **Health (stress)** | 500 | 20,000 | 15.53 ms | 28.78 ms | **26,315 req/s** | 0 |

### Sled vs SQLite Comparison

| Endpoint | Sled | SQLite | Speedup |
|----------|:----:|:------:|:-------:|
| Dashboard Stats | **1.76 ms** | 5.39 ms | **3.1x faster** |
| List Sales | **1.40 ms** | 4.24 ms | **3.0x faster** |
| Stock Movements | **1.48 ms** | 2.97 ms | **2.0x faster** |
| List Products | **2.57 ms** | 10.17 ms | **4.0x faster** |
| Login | **108 ms** | 158 ms | **1.5x faster** |
| Stress p99 | **12.39 ms** | 27.47 ms | **2.2x faster** |

### Sled In-Process Benchmarks

| Operation | Latency | Notes |
|-----------|:-------:|-------|
| **Read by ID** | 17 µs | 500 point lookups |
| **Concurrent reads** | 35 µs | 100 parallel tokio tasks |
| **Stock update** | 68 µs | 1000 read-modify-write |
| **Write product** | 87 µs | 1000 sequential writes |
| **List (filtered)** | 15 ms | Scan 1000, filter, page |

## Security

PCI DSS-aligned. Full audit: [SECURITY.md](./SECURITY.md)

| Layer | Implementation |
|-------|---------------|
| **Authentication** | JWT (HS256) + Argon2id |
| **Authorization** | RBAC (admin/user) |
| **Encryption at Rest** | AES-256-GCM |
| **Encryption in Transit** | TLS 1.2+ |
| **Input Validation** | `validator` crate (regex, length, range) |
| **SQL Injection** | N/A — Sled is key-value, no SQL |
| **Session Management** | JTI tracking with instant revocation |
| **Audit Trail** | All mutations logged |
| **Timing Attacks** | Constant-time comparison |

## Test Coverage

```
33/33 passed — 0 failed

Auth (7)       │ JWT generate/validate, expired, tampered, wrong secret, refresh
Crypto (5)     │ AES-256-GCM roundtrip, unique ciphertext, tamper, wrong key, key length
Validation (10)│ Products, restock, login, SKU regex, sanitize, UUID
Config (3)     │ Valid config, short JWT, wrong AES key
Sled Bench (6) │ Write, read, list, dashboard, stock update, concurrent reads
```

## Tech Stack

| Component | Technology |
|-----------|-----------|
| **Backend** | Rust + tonic (gRPC) |
| **Storage** | Sled (lock-free B+Tree, pure Rust) |
| **Frontend** | Rust → WebAssembly (wasm-pack) |
| **Auth** | JWT + Argon2id |
| **Encryption** | AES-256-GCM |
| **UI** | Vanilla HTML/CSS/JS, Inter + JetBrains Mono |
| **Transport** | gRPC + gRPC-Web |

## Quick Start

### Backend

```bash
cd backend

export JWT_SECRET=$(openssl rand -hex 32)
export JWT_REFRESH_SECRET=$(openssl rand -hex 32)
export AES_KEY=$(openssl rand -hex 16)

cargo run --release --bin server
# → 0.0.0.0:50051 (Sled storage at ./techfix_data)
```

### Frontend

```bash
wasm-pack build --target web --out-dir pkg
python3 serve.py
# → http://localhost:8080
```

### Default Credentials

| Field | Value |
|-------|-------|
| Username | `admin` |
| Password | `admin12345` |

## API Reference

| Service | Method | Auth | Description |
|---------|--------|:----:|-------------|
| `AuthService` | `Login` | No | Get JWT tokens |
| `AuthService` | `RefreshToken` | No | Refresh access token |
| `ProductService` | `Create` | Yes | Create product |
| `ProductService` | `Get` | Yes | Get by ID |
| `ProductService` | `Update` | Yes | Update product |
| `ProductService` | `Delete` | Yes | Delete (if no sales) |
| `ProductService` | `List` | Yes | Search + pagination |
| `SalesService` | `Create` | Yes | Register sale |
| `SalesService` | `List` | Yes | Sales history |
| `StockService` | `Restock` | Yes | Add stock |
| `StockService` | `ListMovements` | Yes | Movement history |
| `DashboardService` | `GetStats` | Yes | Aggregated metrics |
| `HealthService` | `Check` | No | Liveness probe |

## Project Structure

```
techfix-erp/
├── backend/
│   ├── proto/erp.proto
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── auth.rs
│   │   ├── config.rs
│   │   ├── crypto.rs
│   │   ├── db.rs
│   │   ├── error.rs
│   │   ├── middleware.rs
│   │   ├── validation.rs
│   │   ├── tests.rs
│   │   ├── store/
│   │   │   ├── mod.rs            # Store trait
│   │   │   └── sled_store.rs     # Sled implementation
│   │   └── services/
│   └── Cargo.toml
├── src/lib.rs                    # WASM core
├── index.html                    # Frontend
├── SECURITY.md
└── README.md
```

## License

MIT
