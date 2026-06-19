use tonic::{Request, Response, Status};
use crate::{config::AppConfig, db, middleware, store::Store, validation};
use validation::ValidateExt;
use std::sync::Arc;

pub struct StockServiceImpl<S: Store> {
    pub config: Arc<AppConfig>,
    pub store: S,
    pub auth: middleware::AuthInterceptor<S>,
}

#[tonic::async_trait]
impl<S: Store> crate::generated::stock_service_server::StockService for StockServiceImpl<S> {
    async fn restock(
        &self,
        request: Request<crate::generated::RestockRequest>,
    ) -> Result<Response<crate::generated::StockMovement>, Status> {
        let ctx = self.auth.intercept(&request).await?;
        let req = request.into_inner();
        let ip = middleware::extract_ip(&Request::new(()));

        let input = validation::RestockInput {
            product_id: req.product_id,
            quantity: req.quantity,
            notes: validation::sanitize(&req.notes),
        };
        input.validate_input()?;

        let product = self.store.get_product(&input.product_id).await
            .map_err(|_| Status::internal("Database error"))?
            .ok_or_else(|| Status::not_found("Product not found"))?;

        self.store.update_stock(&input.product_id, input.quantity as i64).await
            .map_err(|_| Status::internal("Stock update failed"))?;

        let movement = db::StockMovement {
            id: uuid::Uuid::new_v4().to_string(),
            product_id: input.product_id.clone(),
            product_name: product.name.clone(),
            quantity: input.quantity as i64,
            movement_type: "IN".to_string(),
            notes: input.notes.clone(),
            created_at: chrono::Utc::now().naive_utc(),
        };
        self.store.create_stock_movement(&movement).await
            .map_err(|_| Status::internal("Movement log failed"))?;

        let _ = self.store.audit(&ctx.user_id, "RESTOCK", "product", &input.product_id,
            &format!("qty={}, notes={}", input.quantity, input.notes), &ip).await;

        Ok(Response::new(crate::generated::StockMovement {
            id: movement.id,
            product_id: movement.product_id,
            product_name: movement.product_name,
            quantity: movement.quantity as u32,
            movement_type: movement.movement_type,
            notes: movement.notes,
            created_at: Some(prost_types::Timestamp {
                seconds: movement.created_at.and_utc().timestamp(),
                nanos: 0,
            }),
        }))
    }

    async fn list_movements(
        &self,
        request: Request<crate::generated::ListMovementsRequest>,
    ) -> Result<Response<crate::generated::ListMovementsResponse>, Status> {
        let _ctx = self.auth.intercept(&request).await?;
        let req = request.into_inner();
        let page = req.page.max(1);
        let page_size = req.page_size.clamp(1, 100);
        let offset = ((page - 1) * page_size) as i64;

        let (movements, total) = self.store.list_stock_movements(&req.product_id, offset, page_size as i64).await
            .map_err(|_| Status::internal("Database error"))?;

        Ok(Response::new(crate::generated::ListMovementsResponse {
            movements: movements.iter().map(|m| crate::generated::StockMovement {
                id: m.id.clone(),
                product_id: m.product_id.clone(),
                product_name: m.product_name.clone(),
                quantity: m.quantity as u32,
                movement_type: m.movement_type.clone(),
                notes: m.notes.clone(),
                created_at: Some(prost_types::Timestamp {
                    seconds: m.created_at.and_utc().timestamp(),
                    nanos: 0,
                }),
            }).collect(),
            total: total as u32,
        }))
    }
}
