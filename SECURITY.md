# TechFix ERP — Security Architecture & PCI Compliance

## Security Audit Report

### 1. Authentication & Session Management

| Control | Implementation | PCI Ref |
|---------|---------------|---------|
| Password Hashing | Argon2id with random salt, 3 iterations | Req 8.2.1 |
| JWT Tokens | HS256, 15min access / 7d refresh, unique JTI | Req 8.1 |
| Session Revocation | DB-backed JTI tracking, instant revoke | Req 8.1.8 |
| Failed Login Audit | All failures logged with IP, timestamp | Req 10.2.4 |
| Token Rotation | Refresh tokens rotate on use | Req 8.2.4 |

### 2. Data Protection

| Control | Implementation | PCI Ref |
|---------|---------------|---------|
| Encryption at Rest | AES-256-GCM for sensitive fields | Req 3.4 |
| TLS in Transit | TLS 1.2+ enforced (configurable cert/key) | Req 4.1 |
| Secure Delete | SQLite `PRAGMA secure_delete=ON` | Req 3.1 |
| No Plaintext Secrets | All passwords hashed, keys env-only | Req 3.2 |

### 3. Input Validation & Injection Prevention

| Control | Implementation | PCI Ref |
|---------|---------------|---------|
| Input Validation | `validator` crate with length/range/regex | Req 6.5 |
| SQL Injection | Parameterized queries via `sqlx` | Req 6.5.4 |
| XSS Prevention | Server-side rendering, no user HTML | Req 6.5.7 |
| Control Char Stripping | `sanitize()` removes control characters | Req 6.5 |
| UUID Validation | All IDs validated as UUIDv4 format | — |
| Max Input Length | All fields capped (name:200, sku:50, etc.) | — |

### 4. Authorization & Access Control

| Control | Implementation | PCI Ref |
|---------|---------------|---------|
| Role-Based Access | admin/user roles, checked per endpoint | Req 7.1 |
| Auth Interceptor | Every mutation endpoint requires valid JWT | Req 7.2 |
| Session Validation | JTI checked against DB on every request | Req 8.1 |
| Deletion Protection | Products with sales cannot be deleted | — |

### 5. Audit Logging

| Control | Implementation | PCI Ref |
|---------|---------------|---------|
| Audit Trail | All CRUD operations logged with user, IP, timestamp | Req 10.1 |
| Login Events | Success and failure logged | Req 10.2 |
| Structured Logging | JSON format with trace IDs | Req 10.3 |
| Log Separation | Audit table separate from app data | Req 10.5 |

### 6. Network & Infrastructure

| Control | Implementation | PCI Ref |
|---------|---------------|---------|
| CORS Policy | Configurable allowed origins | Req 1.3 |
| Rate Limiting | Configurable RPS per endpoint | Req 6.6 |
| Connection Limits | Max 256 concurrent per connection | — |
| Request Timeout | 30s server timeout | — |
| gRPC-Web | Browser-compatible transport layer | — |

### 7. Database Hardening

| Control | Implementation |
|---------|---------------|
| WAL Mode | Write-Ahead Logging for concurrency |
| Foreign Keys | Enforced (`PRAGMA foreign_keys=ON`) |
| Busy Timeout | 5s to prevent lock contention |
| Connection Pool | Min 2, Max 10 connections |
| Indexed Queries | Key columns indexed for performance |
| Check Constraints | Stock >= 0, price >= 0, quantity > 0 |

---

## Identified Vulnerabilities & Mitigations

### Fixed in This Build

| # | Vulnerability | Severity | Mitigation |
|---|--------------|----------|------------|
| 1 | Unsafe `static mut` global state | HIGH | Replaced with SQLite-backed state |
| 2 | No authentication | CRITICAL | JWT + Argon2 auth system |
| 3 | No input validation | HIGH | `validator` crate with derive macros |
| 4 | Data in localStorage (XSS accessible) | HIGH | Server-side SQLite storage |
| 5 | No audit trail | MEDIUM | Full audit log table |
| 6 | No session management | HIGH | DB sessions with revocation |
| 7 | Plaintext password storage | CRITICAL | Argon2id hashing |
| 8 | No CORS restrictions | MEDIUM | Configurable CORS policy |
| 9 | No rate limiting | MEDIUM | Configurable RPS limits |
| 10 | No SQL injection protection | HIGH | Parameterized queries (sqlx) |

### Remaining / Future

| # | Item | Priority | Notes |
|---|------|----------|-------|
| 1 | TLS certificate provisioning | HIGH | Configure TLS_CERT_PATH / TLS_KEY_PATH |
| 2 | CSRF tokens for web forms | MEDIUM | gRPC metadata-based CSRF |
| 3 | Account lockout after N failures | MEDIUM | Implement exponential backoff |
| 4 | Secret rotation mechanism | LOW | Add key rotation endpoint |
| 5 | Database encryption (SQLCipher) | LOW | Encrypt entire DB file |
| 6 | WAF / reverse proxy | LOW | Deploy behind nginx/Cloudflare |
| 7 | Penetration testing | HIGH | Required for PCI DSS audit |

---

## Environment Variables

```bash
# Database
DATABASE_URL=sqlite:techfix.db?mode=rwc

# Auth (MUST set in production)
JWT_SECRET=<64-char-hex-string>
JWT_REFRESH_SECRET=<64-char-hex-string>
JWT_ACCESS_TTL=900          # 15 minutes
JWT_REFRESH_TTL=604800      # 7 days

# Encryption (MUST set in production — exactly 32 bytes hex)
AES_KEY=<64-char-hex-string>

# TLS (optional)
TLS_CERT_PATH=/path/to/cert.pem
TLS_KEY_PATH=/path/to/key.pem

# Server
GRPC_PORT=50051
GRPC_WEB_PORT=8443
RATE_LIMIT_RPS=100
LOG_LEVEL=info

# CORS
CORS_ORIGINS=https://yourdomain.com,https://app.yourdomain.com
```

## Default Credentials

**Username:** `admin`
**Password:** `admin12345`

> **CHANGE IMMEDIATELY IN PRODUCTION**
