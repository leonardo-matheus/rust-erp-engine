use std::sync::Arc;
use tonic::transport::Server;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{fmt, EnvFilter};

use techfix_erp_backend::{
    config, generated, middleware, services, store,
    AppConfig, VERSION,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::from_env();
    config.validate().expect("Invalid configuration");

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .json()
        .init();

    tracing::info!("Starting TechFix ERP Server v{}", VERSION);

    let sled_path = std::env::var("SLED_PATH").unwrap_or_else(|_| "./techfix_data".to_string());
    let store = store::sled_store::SledStore::open(&sled_path)?;
    tracing::info!("Sled database opened at {}", sled_path);

    store::sled_store::seed_demo_data(&store).await?;

    let config = Arc::new(config);
    let auth_interceptor = middleware::AuthInterceptor::new(store.clone());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .max_age(std::time::Duration::from_secs(86400));

    let health_svc = generated::health_service_server::HealthServiceServer::new(
        services::health_service::HealthServiceImpl
    );
    let auth_svc = generated::auth_service_server::AuthServiceServer::new(
        services::auth_service::AuthServiceImpl {
            config: config.clone(),
            store: store.clone(),
        }
    );
    let product_svc = generated::product_service_server::ProductServiceServer::new(
        services::product_service::ProductServiceImpl {
            config: config.clone(),
            store: store.clone(),
            auth: auth_interceptor.clone(),
        }
    );
    let sales_svc = generated::sales_service_server::SalesServiceServer::new(
        services::sales_service::SalesServiceImpl {
            config: config.clone(),
            store: store.clone(),
            auth: auth_interceptor.clone(),
        }
    );
    let stock_svc = generated::stock_service_server::StockServiceServer::new(
        services::stock_service::StockServiceImpl {
            config: config.clone(),
            store: store.clone(),
            auth: auth_interceptor.clone(),
        }
    );
    let dashboard_svc = generated::dashboard_service_server::DashboardServiceServer::new(
        services::dashboard_service::DashboardServiceImpl {
            config: config.clone(),
            store: store.clone(),
            auth: auth_interceptor.clone(),
        }
    );

    let addr = format!("0.0.0.0:{}", config.grpc_port).parse()?;
    tracing::info!("gRPC server listening on {}", addr);
    tracing::info!("Storage engine: Sled (lock-free B+Tree, pure Rust)");

    Server::builder()
        .timeout(std::time::Duration::from_secs(30))
        .concurrency_limit_per_connection(256)
        .add_service(health_svc)
        .add_service(auth_svc)
        .add_service(product_svc)
        .add_service(sales_svc)
        .add_service(stock_svc)
        .add_service(dashboard_svc)
        .serve(addr)
        .await?;

    Ok(())
}
