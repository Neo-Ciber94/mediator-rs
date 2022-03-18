use crate::models::product::Product;
use crate::services::redis_service::SharedRedisService;
use mediator::{Request, RequestHandler};
use uuid::Uuid;

pub struct GetProductRequest(pub Uuid);
impl Request<Option<Product>> for GetProductRequest {}

pub struct GetProductRequestHandler(pub SharedRedisService<Product>);
impl RequestHandler<GetProductRequest, Option<Product>> for GetProductRequestHandler {
    fn handle(&mut self, req: GetProductRequest) -> Option<Product> {
        self.0
            .try_lock()
            .expect("Failed to lock redis service")
            .get(req.0.to_string())
            .expect("Failed to get product from redis")
    }
}
