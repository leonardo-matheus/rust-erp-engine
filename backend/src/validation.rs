use crate::error::AppError;
use validator::Validate;

#[derive(Debug, Validate)]
pub struct CreateProductInput {
    #[validate(length(min = 1, max = 200, message = "Name must be 1-200 characters"))]
    pub name: String,

    #[validate(length(min = 1, max = 50, message = "SKU must be 1-50 characters"))]
    #[validate(regex(path = *SKU_REGEX, message = "SKU must be alphanumeric with hyphens"))]
    pub sku: String,

    #[validate(length(min = 1, max = 100, message = "Category must be 1-100 characters"))]
    pub category: String,

    #[validate(range(min = 0.0, max = 999999.99, message = "Price must be 0-999999.99"))]
    pub price: f64,

    #[validate(range(min = 0.0, max = 999999.99, message = "Cost must be 0-999999.99"))]
    pub cost: f64,

    #[validate(range(min = 0, max = 999999, message = "Stock must be 0-999999"))]
    pub stock: u32,

    #[validate(range(min = 0, max = 999999, message = "Min stock must be 0-999999"))]
    pub min_stock: u32,
}

#[derive(Debug, Validate)]
pub struct UpdateProductInput {
    #[validate(length(min = 1, max = 50))]
    pub id: String,

    #[validate(length(min = 1, max = 200))]
    pub name: String,

    #[validate(length(min = 1, max = 50))]
    #[validate(regex(path = *SKU_REGEX))]
    pub sku: String,

    #[validate(length(min = 1, max = 100))]
    pub category: String,

    #[validate(range(min = 0.0, max = 999999.99))]
    pub price: f64,

    #[validate(range(min = 0.0, max = 999999.99))]
    pub cost: f64,

    #[validate(range(min = 0, max = 999999))]
    pub min_stock: u32,
}

#[derive(Debug, Validate)]
pub struct SaleItemInput {
    #[validate(length(min = 1, max = 50))]
    pub product_id: String,

    #[validate(range(min = 1, max = 999999))]
    pub quantity: u32,
}

#[derive(Debug, Validate)]
pub struct CreateSaleInput {
    pub items: Vec<SaleItemInput>,

    #[validate(length(min = 1, max = 200, message = "Customer name required"))]
    pub customer: String,

    #[validate(length(min = 1, max = 50))]
    #[validate(custom(function = "validate_payment_method"))]
    pub payment_method: String,
}

#[derive(Debug, Validate)]
pub struct RestockInput {
    #[validate(length(min = 1, max = 50))]
    pub product_id: String,

    #[validate(range(min = 1, max = 999999))]
    pub quantity: u32,

    #[validate(length(max = 500))]
    pub notes: String,
}

#[derive(Debug, Validate)]
pub struct LoginInput {
    #[validate(length(min = 3, max = 100, message = "Username must be 3-100 characters"))]
    #[validate(regex(path = *USERNAME_REGEX, message = "Username must be alphanumeric"))]
    pub username: String,

    #[validate(length(min = 8, max = 128, message = "Password must be 8-128 characters"))]
    pub password: String,
}

use regex::Regex;
use std::sync::LazyLock;

static SKU_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9\-_]+$").unwrap());
static USERNAME_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_\-\.]+$").unwrap());

fn validate_payment_method(method: &str) -> Result<(), validator::ValidationError> {
    const METHODS: &[&str] = &[
        "Credit Card", "Debit Card", "PIX", "Cash", "Bank Transfer",
    ];
    if METHODS.contains(&method) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid_payment_method"))
    }
}

pub trait ValidateExt {
    fn validate_input(&self) -> Result<(), AppError>;
}

impl<T: Validate> ValidateExt for T {
    fn validate_input(&self) -> Result<(), AppError> {
        self.validate().map_err(|e| {
            let msgs: Vec<String> = e.field_errors().iter().flat_map(|(_, errors)| {
                errors.iter().filter_map(|e| e.message.as_ref().map(|m| m.to_string()))
            }).collect();
            AppError::Validation(msgs.join("; "))
        })
    }
}

/// Sanitize string to prevent injection attacks
pub fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .take(10000)
        .collect()
}

/// Validate UUID format
pub fn validate_uuid(s: &str) -> Result<(), AppError> {
    uuid::Uuid::parse_str(s)
        .map(|_| ())
        .map_err(|_| AppError::Validation(format!("Invalid UUID: {}", s)))
}
