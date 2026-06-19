use tonic::{Request, Response, Status};
use crate::{config::AppConfig, db, middleware, store::Store, validation};
use validation::ValidateExt;
use std::sync::Arc;

pub struct SalesServiceImpl<S: Store> {
    pub config: Arc<AppConfig>,
    pub store: S,
    pub auth: middleware::AuthInterceptor<S>,
}

#[tonic::async_trait]
impl<S: Store> crate::generated::sales_service_server::SalesService for SalesServiceImpl<S> {
    async fn create(
        &self,
        request: Request<crate::generated::CreateSaleRequest>,
    ) -> Result<Response<crate::generated::Sale>, Status> {
        let ctx = self.auth.intercept(&request).await?;
        let req = request.into_inner();
        let ip = middleware::extract_ip(&Request::new(()));

        let customer = validation::sanitize(&req.customer);
        if customer.is_empty() || customer.len() > 200 {
            return Err(Status::invalid_argument("Customer name required (1-200 chars)"));
        }
        let payment = validation::sanitize(&req.payment_method);

        if req.items.is_empty() {
            return Err(Status::invalid_argument("At least one item required"));
        }
        if req.items.len() > 100 {
            return Err(Status::invalid_argument("Too many items (max 100)"));
        }

        let sale_id = uuid::Uuid::new_v4().to_string();
        let mut total = 0.0f64;
        let mut sale_items = Vec::new();

        for item in &req.items {
            validation::validate_uuid(&item.product_id)?;

            let product = self.store.get_product(&item.product_id).await
                .map_err(|_| Status::internal("Database error"))?
                .ok_or_else(|| Status::not_found(format!("Product {} not found", item.product_id)))?;

            if product.stock < item.quantity as i64 {
                return Err(Status::failed_precondition(
                    format!("Insufficient stock for '{}': {} available, {} requested",
                        product.name, product.stock, item.quantity)
                ));
            }

            let subtotal = product.price * item.quantity as f64;
            total += subtotal;

            // Decrement stock
            self.store.update_stock(&item.product_id, -(item.quantity as i64)).await
                .map_err(|_| Status::internal("Stock update failed"))?;

            // Record movement
            let movement = db::StockMovement {
                id: uuid::Uuid::new_v4().to_string(),
                product_id: item.product_id.clone(),
                product_name: product.name.clone(),
                quantity: item.quantity as i64,
                movement_type: "OUT".to_string(),
                notes: format!("Sale to {}", customer),
                created_at: chrono::Utc::now().naive_utc(),
            };
            let _ = self.store.create_stock_movement(&movement).await;

            sale_items.push(db::SaleItem {
                id: uuid::Uuid::new_v4().to_string(),
                sale_id: sale_id.clone(),
                product_id: item.product_id.clone(),
                product_name: product.name.clone(),
                quantity: item.quantity as i64,
                unit_price: product.price,
                subtotal,
            });
        }

        let sale = db::Sale {
            id: sale_id.clone(),
            total,
            customer: customer.clone(),
            payment_method: payment.clone(),
            created_at: chrono::Utc::now().naive_utc(),
        };

        self.store.create_sale(&sale, &sale_items).await
            .map_err(|_| Status::internal("Sale creation failed"))?;

        let _ = self.store.audit(&ctx.user_id, "CREATE", "sale", &sale_id,
            &format!("total={:.2}, items={}", total, sale_items.len()), &ip).await;

        Ok(Response::new(crate::generated::Sale {
            id: sale_id,
            items: sale_items.iter().map(|i| crate::generated::SaleItem {
                product_id: i.product_id.clone(),
                product_name: i.product_name.clone(),
                quantity: i.quantity as u32,
                unit_price: i.unit_price,
                subtotal: i.subtotal,
            }).collect(),
            total,
            customer,
            payment_method: payment,
            created_at: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: 0,
            }),
        }))
    }

    async fn list(
        &self,
        request: Request<crate::generated::ListSalesRequest>,
    ) -> Result<Response<crate::generated::ListSalesResponse>, Status> {
        let _ctx = self.auth.intercept(&request).await?;
        let req = request.into_inner();
        let page = req.page.max(1);
        let page_size = req.page_size.clamp(1, 100);
        let offset = ((page - 1) * page_size) as i64;

        let (sales_data, total) = self.store.list_sales(&req.search, offset, page_size as i64).await
            .map_err(|_| Status::internal("Database error"))?;

        let proto_sales = sales_data.into_iter().map(|(s, items)| {
            crate::generated::Sale {
                id: s.id.clone(),
                items: items.iter().map(|i| crate::generated::SaleItem {
                    product_id: i.product_id.clone(),
                    product_name: i.product_name.clone(),
                    quantity: i.quantity as u32,
                    unit_price: i.unit_price,
                    subtotal: i.subtotal,
                }).collect(),
                total: s.total,
                customer: s.customer.clone(),
                payment_method: s.payment_method.clone(),
                created_at: Some(prost_types::Timestamp {
                    seconds: s.created_at.and_utc().timestamp(),
                    nanos: 0,
                }),
            }
        }).collect();

        Ok(Response::new(crate::generated::ListSalesResponse {
            sales: proto_sales,
            total: total as u32,
        }))
    }
}
