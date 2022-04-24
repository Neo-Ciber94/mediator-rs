use crate::models::product::Product;
use crate::SharedRedisService;
use mediator::{AsyncRequestHandler, Request};
use uuid::Uuid;

pub struct GetProductRequest(pub Uuid);
impl Request<Option<Product>> for GetProductRequest {}

pub struct GetProductRequestHandler(pub SharedRedisService<Product>);

#[mediator::async_trait]
impl AsyncRequestHandler<GetProductRequest, Option<Product>> for GetProductRequestHandler {
    async fn handle(&mut self, req: GetProductRequest) -> Option<Product> {
        let mut lock = self.0.lock().await;

        lock.get(req.0.to_string())
            .await
            .expect("Failed to get product from redis")
    }
}
