use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use chrono::Utc;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub sku: String,
    pub category: String,
    pub price: f64,
    pub cost: f64,
    pub stock: u32,
    pub min_stock: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaleItem {
    pub product_id: String,
    pub product_name: String,
    pub quantity: u32,
    pub unit_price: f64,
    pub subtotal: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Sale {
    pub id: String,
    pub items: Vec<SaleItem>,
    pub total: f64,
    pub customer: String,
    pub payment_method: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StockMovement {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub quantity: u32,
    pub movement_type: String,
    pub notes: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppState {
    pub products: Vec<Product>,
    pub sales: Vec<Sale>,
    pub stock_movements: Vec<StockMovement>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            products: Vec::new(),
            sales: Vec::new(),
            stock_movements: Vec::new(),
        }
    }
}

static mut STATE: Option<AppState> = None;

fn state() -> &'static mut AppState {
    unsafe {
        if STATE.is_none() {
            STATE = Some(AppState::new());
        }
        STATE.as_mut().unwrap()
    }
}

fn save_to_storage() {
    let s = state();
    if let Ok(json) = serde_json::to_string(s) {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("erp_data", &json);
            }
        }
    }
}

fn load_from_storage() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(json)) = storage.get_item("erp_data") {
                if let Ok(loaded) = serde_json::from_str::<AppState>(&json) {
                    unsafe {
                        STATE = Some(loaded);
                    }
                }
            }
        }
    }
}

#[wasm_bindgen(start)]
pub fn init() {
    load_from_storage();
}

#[wasm_bindgen]
pub fn add_product(
    name: String,
    sku: String,
    category: String,
    price: f64,
    cost: f64,
    stock: u32,
    min_stock: u32,
) -> String {
    let now = Utc::now().format(r#"%Y-%m-%d %H:%M:%S"#).to_string();
    let product = Product {
        id: Uuid::new_v4().to_string(),
        name,
        sku,
        category,
        price,
        cost,
        stock,
        min_stock,
        created_at: now.clone(),
        updated_at: now,
    };
    let json = serde_json::to_string(&product).unwrap_or_default();
    state().products.push(product);
    save_to_storage();
    json
}

#[wasm_bindgen]
pub fn update_product(
    id: String,
    name: String,
    sku: String,
    category: String,
    price: f64,
    cost: f64,
    min_stock: u32,
) -> String {
    let now = Utc::now().format(r#"%Y-%m-%d %H:%M:%S"#).to_string();
    let s = state();
    if let Some(p) = s.products.iter_mut().find(|p| p.id == id) {
        p.name = name;
        p.sku = sku;
        p.category = category;
        p.price = price;
        p.cost = cost;
        p.min_stock = min_stock;
        p.updated_at = now;
        let json = serde_json::to_string(p).unwrap_or_default();
        save_to_storage();
        return json;
    }
    "{}".to_string()
}

#[wasm_bindgen]
pub fn delete_product(id: String) -> bool {
    let s = state();
    let len_before = s.products.len();
    s.products.retain(|p| p.id != id);
    let deleted = s.products.len() < len_before;
    if deleted {
        save_to_storage();
    }
    deleted
}

#[wasm_bindgen]
pub fn restock_product(id: String, quantity: u32, notes: String) -> String {
    let now = Utc::now().format(r#"%Y-%m-%d %H:%M:%S"#).to_string();
    let s = state();
    let mut movement = None;
    if let Some(p) = s.products.iter_mut().find(|p| p.id == id) {
        p.stock += quantity;
        p.updated_at = now.clone();
        movement = Some(StockMovement {
            id: Uuid::new_v4().to_string(),
            product_id: p.id.clone(),
            product_name: p.name.clone(),
            quantity,
            movement_type: "IN".to_string(),
            notes,
            created_at: now,
        });
    }
    if let Some(m) = movement {
        let json = serde_json::to_string(&m).unwrap_or_default();
        s.stock_movements.push(m);
        save_to_storage();
        return json;
    }
    "{}".to_string()
}

#[wasm_bindgen]
pub fn register_sale(
    items_json: String,
    customer: String,
    payment_method: String,
) -> String {
    let now = Utc::now().format(r#"%Y-%m-%d %H:%M:%S"#).to_string();
    let raw_items: Vec<SaleItem> = match serde_json::from_str(&items_json) {
        Ok(v) => v,
        Err(_) => return "{}".to_string(),
    };

    let s = state();
    let mut sale_items = Vec::new();
    let mut total = 0.0;

    for item in &raw_items {
        if let Some(p) = s.products.iter_mut().find(|p| p.id == item.product_id) {
            if p.stock >= item.quantity {
                p.stock -= item.quantity;
                let subtotal = item.unit_price * item.quantity as f64;
                total += subtotal;
                sale_items.push(SaleItem {
                    product_id: p.id.clone(),
                    product_name: p.name.clone(),
                    quantity: item.quantity,
                    unit_price: item.unit_price,
                    subtotal,
                });

                s.stock_movements.push(StockMovement {
                    id: Uuid::new_v4().to_string(),
                    product_id: p.id.clone(),
                    product_name: p.name.clone(),
                    quantity: item.quantity,
                    movement_type: "OUT".to_string(),
                    notes: format!("Sale to {}", customer),
                    created_at: now.clone(),
                });
            }
        }
    }

    if sale_items.is_empty() {
        return "{}".to_string();
    }

    let sale = Sale {
        id: Uuid::new_v4().to_string(),
        items: sale_items,
        total,
        customer,
        payment_method,
        created_at: now,
    };
    let json = serde_json::to_string(&sale).unwrap_or_default();
    s.sales.push(sale);
    save_to_storage();
    json
}

#[wasm_bindgen]
pub fn get_products() -> String {
    serde_json::to_string(&state().products).unwrap_or_default()
}

#[wasm_bindgen]
pub fn get_sales() -> String {
    serde_json::to_string(&state().sales).unwrap_or_default()
}

#[wasm_bindgen]
pub fn get_stock_movements() -> String {
    serde_json::to_string(&state().stock_movements).unwrap_or_default()
}

#[wasm_bindgen]
pub fn get_product(id: String) -> String {
    let s = state();
    if let Some(p) = s.products.iter().find(|p| p.id == id) {
        serde_json::to_string(p).unwrap_or_default()
    } else {
        "{}".to_string()
    }
}

#[wasm_bindgen]
pub fn get_dashboard_stats() -> String {
    let s = state();
    let total_products = s.products.len();
    let total_stock: u32 = s.products.iter().map(|p| p.stock).sum();
    let low_stock = s.products.iter().filter(|p| p.stock <= p.min_stock).count();
    let total_sales = s.sales.len();
    let revenue: f64 = s.sales.iter().map(|s| s.total).sum();
    let profit: f64 = s
        .products
        .iter()
        .map(|p| (p.price - p.cost) * p.stock as f64)
        .sum();

    let stats = serde_json::json!({
        "total_products": total_products,
        "total_stock": total_stock,
        "low_stock": low_stock,
        "total_sales": total_sales,
        "revenue": revenue,
        "estimated_profit": profit,
    });
    stats.to_string()
}

#[wasm_bindgen]
pub fn clear_all_data() {
    let s = state();
    s.products.clear();
    s.sales.clear();
    s.stock_movements.clear();
    save_to_storage();
}

#[wasm_bindgen]
pub fn load_demo_data() {
    let s = state();
    if !s.products.is_empty() {
        return;
    }

    let demo_products: Vec<(&str, &str, &str, f64, f64, u32, u32)> = vec![
        (r#"Notebook Pro 15"#, r#"NB-001"#, r#"Electronics"#, 4599.99, 3200.00, 24, 5),
        (r#"Wireless Mouse"#, r#"MS-010"#, r#"Accessories"#, 149.90, 65.00, 150, 20),
        (r#"USB-C Hub 7-in-1"#, r#"HB-003"#, r#"Accessories"#, 249.90, 110.00, 80, 15),
        (r#"Mechanical Keyboard"#, r#"KB-007"#, r#"Accessories"#, 599.99, 280.00, 45, 10),
        (r#"27 4K Monitor"#, r#"MN-027"#, r#"Electronics"#, 2899.99, 1800.00, 18, 5),
        (r#"Webcam HD 1080p"#, r#"WC-002"#, r#"Accessories"#, 299.90, 120.00, 60, 10),
        (r#"External SSD 1TB"#, r#"SSD-1T"#, r#"Storage"#, 499.99, 250.00, 95, 15),
        (r#"Laptop Stand"#, r#"LS-001"#, r#"Accessories"#, 189.90, 70.00, 200, 25),
        (r#"Noise Cancelling Headphones"#, r#"HP-005"#, r#"Audio"#, 1299.99, 650.00, 30, 8),
        (r#"Desk Lamp LED"#, r#"DL-003"#, r#"Office"#, 229.90, 95.00, 70, 12),
    ];

    let now = Utc::now().format(r#"%Y-%m-%d %H:%M:%S"#).to_string();
    for (name, sku, cat, price, cost, stock, min) in demo_products {
        s.products.push(Product {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            sku: sku.to_string(),
            category: cat.to_string(),
            price,
            cost,
            stock,
            min_stock: min,
            created_at: now.clone(),
            updated_at: now.clone(),
        });
    }

    let demo_sales: Vec<(&str, &str, f64)> = vec![
        (r#"Maria Silva"#, r#"Credit Card"#, 1859.89),
        (r#"Joao Santos"#, r#"PIX"#, 749.89),
        (r#"Ana Oliveira"#, r#"Debit Card"#, 3199.89),
        (r#"Carlos Lima"#, r#"Credit Card"#, 4599.99),
        (r#"Fernanda Costa"#, r#"PIX"#, 599.99),
    ];

    for (customer, method, total) in demo_sales {
        let product = &s.products[rand_idx(total as usize) % s.products.len()];
        s.sales.push(Sale {
            id: Uuid::new_v4().to_string(),
            items: vec![SaleItem {
                product_id: product.id.clone(),
                product_name: product.name.clone(),
                quantity: 1,
                unit_price: total,
                subtotal: total,
            }],
            total,
            customer: customer.to_string(),
            payment_method: method.to_string(),
            created_at: now.clone(),
        });
    }

    save_to_storage();
}

fn rand_idx(seed: usize) -> usize {
    let h = seed.wrapping_mul(2654435761).wrapping_add(1013904223);
    h % 10
}
