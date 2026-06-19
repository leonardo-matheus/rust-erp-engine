use tonic::{Request, Response, Status};
use crate::{config::AppConfig, db, middleware, store::Store, validation};
use validation::ValidateExt;
use std::sync::Arc;

pub struct ProductServiceImpl<S: Store> {
    pub config: Arc<AppConfig>,
    pub store: S,
    pub auth: middleware::AuthInterceptor<S>,
}

#[tonic::async_trait]
impl<S: Store> crate::generated::product_service_server::ProductService for ProductServiceImpl<S> {
    async fn create(
        &self,
        request: Request<crate::generated::CreateProductRequest>,
    ) -> Result<Response<crate::generated::Product>, Status> {
        let ctx = self.auth.intercept(&request).await?;
        let req = request.into_inner();
        let ip = middleware::extract_ip(&Request::new(()));

        let input = validation::CreateProductInput {
            name: validation::sanitize(&req.name),
            sku: validation::sanitize(&req.sku),
            category: validation::sanitize(&req.category),
            price: req.price,
            cost: req.cost,
            stock: req.stock,
            min_stock: req.min_stock,
        };
        input.validate_input()?;

        if input.price < input.cost {
            return Err(Status::invalid_argument("Price cannot be less than cost"));
        }

        let now = chrono::Utc::now().naive_utc();
        let p = db::Product {
            id: uuid::Uuid::new_v4().to_string(),
            name: input.name,
            sku: input.sku,
            category: input.category,
            price: input.price,
            cost: input.cost,
            stock: input.stock as i64,
            min_stock: input.min_stock as i64,
            created_at: now,
            updated_at: now,
        };

        self.store.create_product(&p).await.map_err(|e| match e {
            crate::error::AppError::Conflict(msg) => Status::already_exists(msg),
            _ => Status::internal("Database error"),
        })?;

        let _ = self.store.audit(&ctx.user_id, "CREATE", "product", &p.id, &format!("name={}", p.name), &ip).await;

        Ok(Response::new(crate::generated::Product {
            id: p.id, name: p.name, sku: p.sku, category: p.category,
            price: p.price, cost: p.cost,
            stock: p.stock as u32, min_stock: p.min_stock as u32,
            created_at: Some(prost_types::Timestamp { seconds: now.and_utc().timestamp(), nanos: 0 }),
            updated_at: Some(prost_types::Timestamp { seconds: now.and_utc().timestamp(), nanos: 0 }),
        }))
    }

    async fn get(
        &self,
        request: Request<crate::generated::GetProductRequest>,
    ) -> Result<Response<crate::generated::Product>, Status> {
        let _ctx = self.auth.intercept(&request).await?;
        let id = request.into_inner().id;
        validation::validate_uuid(&id)?;

        let p = self.store.get_product(&id).await
            .map_err(|_| Status::internal("Database error"))?
            .ok_or_else(|| Status::not_found("Product not found"))?;

        Ok(Response::new(db_product_to_proto(&p)))
    }

    async fn update(
        &self,
        request: Request<crate::generated::UpdateProductRequest>,
    ) -> Result<Response<crate::generated::Product>, Status> {
        let ctx = self.auth.intercept(&request).await?;
        let req = request.into_inner();
        let ip = middleware::extract_ip(&Request::new(()));

        let input = validation::UpdateProductInput {
            id: req.id,
            name: validation::sanitize(&req.name),
            sku: validation::sanitize(&req.sku),
            category: validation::sanitize(&req.category),
            price: req.price,
            cost: req.cost,
            min_stock: req.min_stock,
        };
        input.validate_input()?;

        let mut p = self.store.get_product(&input.id).await
            .map_err(|_| Status::internal("Database error"))?
            .ok_or_else(|| Status::not_found("Product not found"))?;

        p.name = input.name;
        p.sku = input.sku;
        p.category = input.category;
        p.price = input.price;
        p.cost = input.cost;
        p.min_stock = input.min_stock as i64;
        p.updated_at = chrono::Utc::now().naive_utc();

        self.store.update_product(&p).await
            .map_err(|_| Status::internal("Database error"))?;

        let _ = self.store.audit(&ctx.user_id, "UPDATE", "product", &p.id, &format!("name={}", p.name), &ip).await;

        Ok(Response::new(db_product_to_proto(&p)))
    }

    async fn delete(
        &self,
        request: Request<crate::generated::DeleteProductRequest>,
    ) -> Result<Response<()>, Status> {
        let ctx = self.auth.intercept(&request).await?;
        let id = request.into_inner().id;
        validation::validate_uuid(&id)?;
        let ip = middleware::extract_ip(&Request::new(()));

        let count = self.store.count_sales_for_product(&id).await
            .map_err(|_| Status::internal("Database error"))?;
        if count > 0 {
            return Err(Status::failed_precondition("Cannot delete product with existing sales"));
        }

        let deleted = self.store.delete_product(&id).await
            .map_err(|_| Status::internal("Database error"))?;
        if !deleted {
            return Err(Status::not_found("Product not found"));
        }

        let _ = self.store.audit(&ctx.user_id, "DELETE", "product", &id, "", &ip).await;
        Ok(Response::new(()))
    }

    async fn list(
        &self,
        request: Request<crate::generated::ListProductsRequest>,
    ) -> Result<Response<crate::generated::ListProductsResponse>, Status> {
        let _ctx = self.auth.intercept(&request).await?;
        let req = request.into_inner();
        let page = req.page.max(1);
        let page_size = req.page_size.clamp(1, 100);
        let offset = ((page - 1) * page_size) as i64;

        let (products, total) = self.store.list_products(&req.search, &req.category, offset, page_size as i64).await
            .map_err(|_| Status::internal("Database error"))?;

        Ok(Response::new(crate::generated::ListProductsResponse {
            products: products.iter().map(db_product_to_proto).collect(),
            total: total as u32,
        }))
    }
}

fn db_product_to_proto(p: &db::Product) -> crate::generated::Product {
    crate::generated::Product {
        id: p.id.clone(),
        name: p.name.clone(),
        sku: p.sku.clone(),
        category: p.category.clone(),
        price: p.price,
        cost: p.cost,
        stock: p.stock as u32,
        min_stock: p.min_stock as u32,
        created_at: Some(prost_types::Timestamp {
            seconds: p.created_at.and_utc().timestamp(),
            nanos: 0,
        }),
        updated_at: Some(prost_types::Timestamp {
            seconds: p.updated_at.and_utc().timestamp(),
            nanos: 0,
        }),
    }
}
