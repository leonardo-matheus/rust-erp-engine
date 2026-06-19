use super::{DashboardStats, Store};
use crate::db;
use crate::error::AppError;
use async_trait::async_trait;
use sled::Db;
use std::sync::Arc;

#[derive(Clone)]
pub struct SledStore {
    db: Db,
}

impl SledStore {
    pub fn open(path: &str) -> Result<Self, AppError> {
        let db = sled::open(path)
            .map_err(|e| AppError::Internal(format!("Sled open: {}", e)))?;
        Ok(Self { db })
    }

    fn tree(&self, name: &str) -> sled::Tree {
        self.db.open_tree(name)
            .unwrap_or_else(|_| panic!("Failed to open tree: {}", name))
    }

    fn next_id(&self, counter: &str) -> Result<u64, AppError> {
        self.db.generate_id()
            .map_err(|e| AppError::Internal(format!("ID gen: {}", e)))
    }
}

#[async_trait]
impl Store for SledStore {
    // ── PRODUCTS ──

    async fn create_product(&self, p: &db::Product) -> Result<(), AppError> {
        let tree = self.tree("products");
        let sku_tree = self.tree("product_skus");

        // Check SKU uniqueness
        if sku_tree.get(&p.sku).map_err(|e| AppError::Internal(e.to_string()))?.is_some() {
            return Err(AppError::Conflict(format!("SKU '{}' already exists", p.sku)));
        }

        let data = serde_json::to_vec(p).map_err(|e| AppError::Internal(e.to_string()))?;
        tree.insert(p.id.as_bytes(), data).map_err(|e| AppError::Internal(e.to_string()))?;
        sku_tree.insert(p.sku.as_bytes(), p.id.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn get_product(&self, id: &str) -> Result<Option<db::Product>, AppError> {
        let tree = self.tree("products");
        match tree.get(id.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))? {
            Some(data) => {
                let p: db::Product = serde_json::from_slice(&data)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(Some(p))
            }
            None => Ok(None),
        }
    }

    async fn update_product(&self, p: &db::Product) -> Result<bool, AppError> {
        let tree = self.tree("products");
        if tree.get(p.id.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))?.is_none() {
            return Ok(false);
        }
        let data = serde_json::to_vec(p).map_err(|e| AppError::Internal(e.to_string()))?;
        tree.insert(p.id.as_bytes(), data).map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(true)
    }

    async fn delete_product(&self, id: &str) -> Result<bool, AppError> {
        let tree = self.tree("products");
        // Get product to remove SKU mapping
        if let Some(data) = tree.get(id.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))? {
            let p: db::Product = serde_json::from_slice(&data)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let sku_tree = self.tree("product_skus");
            sku_tree.remove(p.sku.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))?;
            tree.remove(id.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn list_products(&self, search: &str, category: &str, offset: i64, limit: i64) -> Result<(Vec<db::Product>, i64), AppError> {
        let tree = self.tree("products");
        let search_lower = search.to_lowercase();
        let cat_lower = category.to_lowercase();

        let mut all: Vec<db::Product> = Vec::new();
        for entry in tree.iter() {
            let (_, data) = entry.map_err(|e| AppError::Internal(e.to_string()))?;
            let p: db::Product = serde_json::from_slice(&data)
                .map_err(|e| AppError::Internal(e.to_string()))?;

            let matches_search = search.is_empty()
                || p.name.to_lowercase().contains(&search_lower)
                || p.sku.to_lowercase().contains(&search_lower);
            let matches_cat = category.is_empty()
                || p.category.to_lowercase().contains(&cat_lower);

            if matches_search && matches_cat {
                all.push(p);
            }
        }

        let total = all.len() as i64;
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let page: Vec<db::Product> = all.into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        Ok((page, total))
    }

    async fn get_product_by_sku(&self, sku: &str) -> Result<Option<db::Product>, AppError> {
        let sku_tree = self.tree("product_skus");
        match sku_tree.get(sku.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))? {
            Some(id_bytes) => {
                let id = String::from_utf8_lossy(&id_bytes).to_string();
                self.get_product(&id).await
            }
            None => Ok(None),
        }
    }

    // ── SALES ──

    async fn create_sale(&self, sale: &db::Sale, items: &[db::SaleItem]) -> Result<(), AppError> {
        let sale_tree = self.tree("sales");
        let item_tree = self.tree("sale_items");

        let sale_data = serde_json::to_vec(sale).map_err(|e| AppError::Internal(e.to_string()))?;
        sale_tree.insert(sale.id.as_bytes(), sale_data).map_err(|e| AppError::Internal(e.to_string()))?;

        for item in items {
            let key = format!("{}:{}", sale.id, item.product_id);
            let item_data = serde_json::to_vec(item).map_err(|e| AppError::Internal(e.to_string()))?;
            item_tree.insert(key.as_bytes(), item_data).map_err(|e| AppError::Internal(e.to_string()))?;
        }

        Ok(())
    }

    async fn list_sales(&self, search: &str, offset: i64, limit: i64) -> Result<(Vec<(db::Sale, Vec<db::SaleItem>)>, i64), AppError> {
        let sale_tree = self.tree("sales");
        let item_tree = self.tree("sale_items");
        let search_lower = search.to_lowercase();

        let mut all: Vec<db::Sale> = Vec::new();
        for entry in sale_tree.iter() {
            let (_, data) = entry.map_err(|e| AppError::Internal(e.to_string()))?;
            let s: db::Sale = serde_json::from_slice(&data)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            if search.is_empty() || s.customer.to_lowercase().contains(&search_lower) {
                all.push(s);
            }
        }

        let total = all.len() as i64;
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let page: Vec<db::Sale> = all.into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        let mut result = Vec::new();
        for sale in page {
            let mut items = Vec::new();
            for entry in item_tree.scan_prefix(sale.id.as_bytes()) {
                let (_, data) = entry.map_err(|e| AppError::Internal(e.to_string()))?;
                let item: db::SaleItem = serde_json::from_slice(&data)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                items.push(item);
            }
            result.push((sale, items));
        }

        Ok((result, total))
    }

    async fn count_sales_for_product(&self, product_id: &str) -> Result<i64, AppError> {
        let item_tree = self.tree("sale_items");
        let mut count = 0i64;
        for entry in item_tree.iter() {
            let (_, data) = entry.map_err(|e| AppError::Internal(e.to_string()))?;
            let item: db::SaleItem = serde_json::from_slice(&data)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            if item.product_id == product_id {
                count += 1;
            }
        }
        Ok(count)
    }

    // ── STOCK ──

    async fn update_stock(&self, product_id: &str, delta: i64) -> Result<bool, AppError> {
        let tree = self.tree("products");
        if let Some(data) = tree.get(product_id.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))? {
            let mut p: db::Product = serde_json::from_slice(&data)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            if p.stock + delta < 0 {
                return Err(AppError::Validation("Insufficient stock".into()));
            }
            p.stock += delta;
            p.updated_at = chrono::Utc::now().naive_utc();
            let new_data = serde_json::to_vec(&p).map_err(|e| AppError::Internal(e.to_string()))?;
            tree.insert(product_id.as_bytes(), new_data).map_err(|e| AppError::Internal(e.to_string()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn create_stock_movement(&self, m: &db::StockMovement) -> Result<(), AppError> {
        let tree = self.tree("stock_movements");
        let data = serde_json::to_vec(m).map_err(|e| AppError::Internal(e.to_string()))?;
        tree.insert(m.id.as_bytes(), data).map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn list_stock_movements(&self, product_id: &str, offset: i64, limit: i64) -> Result<(Vec<db::StockMovement>, i64), AppError> {
        let tree = self.tree("stock_movements");
        let mut all: Vec<db::StockMovement> = Vec::new();

        for entry in tree.iter() {
            let (_, data) = entry.map_err(|e| AppError::Internal(e.to_string()))?;
            let m: db::StockMovement = serde_json::from_slice(&data)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            if product_id.is_empty() || m.product_id == product_id {
                all.push(m);
            }
        }

        let total = all.len() as i64;
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let page = all.into_iter().skip(offset as usize).take(limit as usize).collect();
        Ok((page, total))
    }

    // ── DASHBOARD ──

    async fn dashboard_stats(&self) -> Result<DashboardStats, AppError> {
        let prod_tree = self.tree("products");
        let sale_tree = self.tree("sales");

        let mut total_products = 0u32;
        let mut total_stock = 0u32;
        let mut low_stock = 0u32;
        let mut estimated_profit = 0.0f64;

        for entry in prod_tree.iter() {
            let (_, data) = entry.map_err(|e| AppError::Internal(e.to_string()))?;
            let p: db::Product = serde_json::from_slice(&data)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            total_products += 1;
            total_stock += p.stock as u32;
            if p.stock <= p.min_stock {
                low_stock += 1;
            }
            estimated_profit += (p.price - p.cost) * p.stock as f64;
        }

        let mut total_sales = 0u32;
        let mut revenue = 0.0f64;
        for entry in sale_tree.iter() {
            let (_, data) = entry.map_err(|e| AppError::Internal(e.to_string()))?;
            let s: db::Sale = serde_json::from_slice(&data)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            total_sales += 1;
            revenue += s.total;
        }

        Ok(DashboardStats {
            total_products,
            total_stock,
            low_stock,
            total_sales,
            revenue,
            estimated_profit,
        })
    }

    // ── USERS ──

    async fn get_user_by_username(&self, username: &str) -> Result<Option<db::User>, AppError> {
        let tree = self.tree("users");
        let idx = self.tree("user_idx");
        match idx.get(username.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))? {
            Some(id_bytes) => {
                let id = String::from_utf8_lossy(&id_bytes).to_string();
                match tree.get(id.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))? {
                    Some(data) => {
                        let u: db::User = serde_json::from_slice(&data)
                            .map_err(|e| AppError::Internal(e.to_string()))?;
                        Ok(Some(u))
                    }
                    None => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    // ── SESSIONS ──

    async fn create_session(&self, user_id: &str, jti: &str, expires_at: &str, ip: &str) -> Result<(), AppError> {
        let tree = self.tree("sessions");
        let session = serde_json::json!({
            "user_id": user_id,
            "jti": jti,
            "expires_at": expires_at,
            "ip": ip,
            "revoked": false
        });
        let data = serde_json::to_vec(&session).map_err(|e| AppError::Internal(e.to_string()))?;
        tree.insert(jti.as_bytes(), data).map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn is_session_valid(&self, jti: &str) -> Result<bool, AppError> {
        let tree = self.tree("sessions");
        match tree.get(jti.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))? {
            Some(data) => {
                let s: serde_json::Value = serde_json::from_slice(&data)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                let revoked = s["revoked"].as_bool().unwrap_or(true);
                Ok(!revoked)
            }
            None => Ok(false),
        }
    }

    async fn revoke_session(&self, jti: &str) -> Result<(), AppError> {
        let tree = self.tree("sessions");
        if let Some(data) = tree.get(jti.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))? {
            let mut s: serde_json::Value = serde_json::from_slice(&data)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            s["revoked"] = serde_json::Value::Bool(true);
            let new_data = serde_json::to_vec(&s).map_err(|e| AppError::Internal(e.to_string()))?;
            tree.insert(jti.as_bytes(), new_data).map_err(|e| AppError::Internal(e.to_string()))?;
        }
        Ok(())
    }

    // ── AUDIT ──

    async fn audit(&self, user_id: &str, action: &str, resource: &str, resource_id: &str, details: &str, ip: &str) -> Result<(), AppError> {
        let tree = self.tree("audit");
        let id = uuid::Uuid::new_v4().to_string();
        let entry = serde_json::json!({
            "id": id,
            "user_id": user_id,
            "action": action,
            "resource": resource,
            "resource_id": resource_id,
            "details": details,
            "ip": ip,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        let data = serde_json::to_vec(&entry).map_err(|e| AppError::Internal(e.to_string()))?;
        tree.insert(id.as_bytes(), data).map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }
}

pub async fn seed_demo_data(store: &SledStore) -> Result<(), AppError> {
    // Check if already seeded
    if store.get_user_by_username("admin").await?.is_some() {
        return Ok(());
    }

    use crate::auth;
    let hash = auth::hash_password("admin12345").await?;
    let admin = db::User {
        id: uuid::Uuid::new_v4().to_string(),
        username: "admin".to_string(),
        password_hash: hash,
        role: "admin".to_string(),
        is_active: true,
        created_at: chrono::Utc::now().naive_utc(),
    };

    let user_tree = store.tree("users");
    let idx_tree = store.tree("user_idx");
    let data = serde_json::to_vec(&admin).map_err(|e| AppError::Internal(e.to_string()))?;
    user_tree.insert(admin.id.as_bytes(), data).map_err(|e| AppError::Internal(e.to_string()))?;
    idx_tree.insert(admin.username.as_bytes(), admin.id.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))?;

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

    let now = chrono::Utc::now().naive_utc();
    for (name, sku, cat, price, cost, stock, min_stock) in products {
        let p = db::Product {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            sku: sku.to_string(),
            category: cat.to_string(),
            price,
            cost,
            stock,
            min_stock,
            created_at: now,
            updated_at: now,
        };
        store.create_product(&p).await?;
    }

    tracing::info!("Demo data seeded to Sled (admin/admin12345)");
    Ok(())
}
