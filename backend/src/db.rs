use chrono::NaiveDateTime;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use crate::error::AppError;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub sku: String,
    pub category: String,
    pub price: f64,
    pub cost: f64,
    pub stock: i64,
    pub min_stock: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Sale {
    pub id: String,
    pub total: f64,
    pub customer: String,
    pub payment_method: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct SaleItem {
    pub id: String,
    pub sale_id: String,
    pub product_id: String,
    pub product_name: String,
    pub quantity: i64,
    pub unit_price: f64,
    pub subtotal: f64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct StockMovement {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub quantity: i64,
    pub movement_type: String,
    pub notes: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub jti: String,
    pub expires_at: NaiveDateTime,
    pub ip_address: String,
    pub user_agent: String,
    pub revoked: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub user_id: String,
    pub action: String,
    pub resource: String,
    pub resource_id: String,
    pub details: String,
    pub ip_address: String,
    pub created_at: NaiveDateTime,
}

pub async fn init_db(database_url: &str) -> Result<SqlitePool, AppError> {
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(std::time::Duration::from_secs(300))
        .connect(database_url)
        .await
        .map_err(AppError::Database)?;

    sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await?;
    sqlx::query("PRAGMA synchronous=NORMAL").execute(&pool).await?;
    sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;
    sqlx::query("PRAGMA busy_timeout=5000").execute(&pool).await?;
    sqlx::query("PRAGMA secure_delete=ON").execute(&pool).await?;

    run_migrations(&pool).await?;

    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS products (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            sku TEXT NOT NULL UNIQUE,
            category TEXT NOT NULL,
            price REAL NOT NULL CHECK(price >= 0),
            cost REAL NOT NULL CHECK(cost >= 0),
            stock INTEGER NOT NULL DEFAULT 0 CHECK(stock >= 0),
            min_stock INTEGER NOT NULL DEFAULT 0 CHECK(min_stock >= 0),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS sales (
            id TEXT PRIMARY KEY,
            total REAL NOT NULL CHECK(total >= 0),
            customer TEXT NOT NULL,
            payment_method TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS sale_items (
            id TEXT PRIMARY KEY,
            sale_id TEXT NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
            product_id TEXT NOT NULL REFERENCES products(id),
            product_name TEXT NOT NULL,
            quantity INTEGER NOT NULL CHECK(quantity > 0),
            unit_price REAL NOT NULL CHECK(unit_price >= 0),
            subtotal REAL NOT NULL CHECK(subtotal >= 0)
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS stock_movements (
            id TEXT PRIMARY KEY,
            product_id TEXT NOT NULL REFERENCES products(id),
            product_name TEXT NOT NULL,
            quantity INTEGER NOT NULL CHECK(quantity > 0),
            movement_type TEXT NOT NULL CHECK(movement_type IN ('IN', 'OUT')),
            notes TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id),
            jti TEXT NOT NULL UNIQUE,
            expires_at TEXT NOT NULL,
            ip_address TEXT NOT NULL DEFAULT '',
            user_agent TEXT NOT NULL DEFAULT '',
            revoked INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS audit_log (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL DEFAULT '',
            action TEXT NOT NULL,
            resource TEXT NOT NULL,
            resource_id TEXT NOT NULL DEFAULT '',
            details TEXT NOT NULL DEFAULT '',
            ip_address TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
    "#).execute(pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_products_sku ON products(sku)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_products_category ON products(category)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sales_created ON sales(created_at)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_stock_movements_product ON stock_movements(product_id)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_log_user ON audit_log(user_id)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_log_action ON audit_log(action)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_jti ON sessions(jti)").execute(pool).await?;

    Ok(())
}

pub async fn seed_demo_data(pool: &SqlitePool) -> Result<(), AppError> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool).await?;
    if count.0 > 0 { return Ok(()); }

    use crate::auth;
    let admin_hash = auth::hash_password("admin12345").await?;

    sqlx::query("INSERT INTO users (id, username, password_hash, role) VALUES (?, ?, ?, ?)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind("admin")
        .bind(&admin_hash)
        .bind("admin")
        .execute(pool).await?;

    let products: Vec<(&str, &str, &str, f64, f64, i64, i64)> = vec![
        ("Notebook Pro 15", "NB-001", "Electronics", 4599.99, 3200.00, 24, 5),
        ("Wireless Mouse", "MS-010", "Accessories", 149.90, 65.00, 150, 20),
        ("USB-C Hub 7-in-1", "HB-003", "Accessories", 249.90, 110.00, 80, 15),
        ("Mechanical Keyboard", "KB-007", "Accessories", 599.99, 280.00, 45, 10),
        ("4K Monitor 27", "MN-027", "Electronics", 2899.99, 1800.00, 18, 5),
        ("Webcam HD 1080p", "WC-002", "Accessories", 299.90, 120.00, 60, 10),
        ("External SSD 1TB", "SSD-1T", "Storage", 499.99, 250.00, 95, 15),
        ("Laptop Stand", "LS-001", "Accessories", 189.90, 70.00, 200, 25),
        ("Noise Cancelling HP", "HP-005", "Audio", 1299.99, 650.00, 30, 8),
        ("Desk Lamp LED", "DL-003", "Office", 229.90, 95.00, 70, 12),
    ];

    for (name, sku, cat, price, cost, stock, min) in products {
        sqlx::query(
            "INSERT INTO products (id, name, sku, category, price, cost, stock, min_stock) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(name).bind(sku).bind(cat)
        .bind(price).bind(cost).bind(stock).bind(min)
        .execute(pool).await?;
    }

    tracing::info!("Demo data seeded (admin/admin12345)");
    Ok(())
}

pub async fn audit(
    pool: &SqlitePool,
    user_id: &str,
    action: &str,
    resource: &str,
    resource_id: &str,
    details: &str,
    ip: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_log (id, user_id, action, resource, resource_id, details, ip_address) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(user_id).bind(action).bind(resource)
    .bind(resource_id).bind(details).bind(ip)
    .execute(pool).await?;
    Ok(())
}
