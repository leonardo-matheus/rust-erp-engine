pub mod auth;
pub mod config;
pub mod crypto;
pub mod db;
pub mod error;
pub mod middleware;
pub mod services;
pub mod store;
pub mod validation;

pub mod generated {
    tonic::include_proto!("techfix.erp");
}

pub use config::AppConfig;
pub use error::AppError;

pub const VERSION: &str = "1.0.0";

#[cfg(test)]
mod tests;
