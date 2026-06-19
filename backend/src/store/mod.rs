pub mod sled_store;

use crate::db;
use crate::error::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait Store: Send + Sync + Clone + 'static {
    // Products
    async fn create_product(&self, p: &db::Product) -> Result<(), AppError>;
    async fn get_product(&self, id: &str) -> Result<Option<db::Product>, AppError>;
    async fn update_product(&self, p: &db::Product) -> Result<bool, AppError>;
    async fn delete_product(&self, id: &str) -> Result<bool, AppError>;
    async fn list_products(&self, search: &str, category: &str, offset: i64, limit: i64) -> Result<(Vec<db::Product>, i64), AppError>;
    async fn get_product_by_sku(&self, sku: &str) -> Result<Option<db::Product>, AppError>;

    // Sales
    async fn create_sale(&self, sale: &db::Sale, items: &[db::SaleItem]) -> Result<(), AppError>;
    async fn list_sales(&self, search: &str, offset: i64, limit: i64) -> Result<(Vec<(db::Sale, Vec<db::SaleItem>)>, i64), AppError>;
    async fn count_sales_for_product(&self, product_id: &str) -> Result<i64, AppError>;

    // Stock
    async fn update_stock(&self, product_id: &str, delta: i64) -> Result<bool, AppError>;
    async fn create_stock_movement(&self, m: &db::StockMovement) -> Result<(), AppError>;
    async fn list_stock_movements(&self, product_id: &str, offset: i64, limit: i64) -> Result<(Vec<db::StockMovement>, i64), AppError>;

    // Dashboard
    async fn dashboard_stats(&self) -> Result<DashboardStats, AppError>;

    // Users
    async fn get_user_by_username(&self, username: &str) -> Result<Option<db::User>, AppError>;

    // Sessions
    async fn create_session(&self, user_id: &str, jti: &str, expires_at: &str, ip: &str) -> Result<(), AppError>;
    async fn is_session_valid(&self, jti: &str) -> Result<bool, AppError>;
    async fn revoke_session(&self, jti: &str) -> Result<(), AppError>;

    // Audit
    async fn audit(&self, user_id: &str, action: &str, resource: &str, resource_id: &str, details: &str, ip: &str) -> Result<(), AppError>;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DashboardStats {
    pub total_products: u32,
    pub total_stock: u32,
    pub low_stock: u32,
    pub total_sales: u32,
    pub revenue: f64,
    pub estimated_profit: f64,
}
