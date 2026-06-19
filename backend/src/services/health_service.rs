use tonic::{Request, Response, Status};
use crate::VERSION;

pub struct HealthServiceImpl;

#[tonic::async_trait]
impl crate::generated::health_service_server::HealthService for HealthServiceImpl {
    async fn check(
        &self,
        _request: Request<()>,
    ) -> Result<Response<crate::generated::HealthResponse>, Status> {
        Ok(Response::new(crate::generated::HealthResponse {
            status: "SERVING".to_string(),
            version: VERSION.to_string(),
            uptime: None,
        }))
    }
}
