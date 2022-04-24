use crate::models::product::Product;
use crate::SharedRedisService;
use mediator::{AsyncRequestHandler, Request};

pub struct GetAllProductsRequest;
impl Request<Vec<Product>> for GetAllProductsRequest {}

pub struct GetAllProductsRequestHandler(pub SharedRedisService<Product>);
#[mediator::async_trait]
impl AsyncRequestHandler<GetAllProductsRequest, Vec<Product>> for GetAllProductsRequestHandler {
    async fn handle(&mut self, _: GetAllProductsRequest) -> Vec<Product> {
        let mut lock = self.0.lock().await;
        lock.get_all().await.expect("Failed to get all products")
    }
}
