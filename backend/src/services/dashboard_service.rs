use tonic::{Request, Response, Status};
use crate::{config::AppConfig, middleware, store::Store};
use std::sync::Arc;

pub struct DashboardServiceImpl<S: Store> {
    pub config: Arc<AppConfig>,
    pub store: S,
    pub auth: middleware::AuthInterceptor<S>,
}

#[tonic::async_trait]
impl<S: Store> crate::generated::dashboard_service_server::DashboardService for DashboardServiceImpl<S> {
    async fn get_stats(
        &self,
        request: Request<()>,
    ) -> Result<Response<crate::generated::DashboardStats>, Status> {
        let _ctx = self.auth.intercept(&request).await?;

        let stats = self.store.dashboard_stats().await
            .map_err(|_| Status::internal("Database error"))?;

        Ok(Response::new(crate::generated::DashboardStats {
            total_products: stats.total_products,
            total_stock: stats.total_stock,
            low_stock: stats.low_stock,
            total_sales: stats.total_sales,
            revenue: stats.revenue,
            estimated_profit: stats.estimated_profit,
        }))
    }
}
