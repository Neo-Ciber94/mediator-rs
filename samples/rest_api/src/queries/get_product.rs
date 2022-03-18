use crate::models::Product;
use crate::BoxedProductService;
use mediator::{Request, RequestHandler};
use uuid::Uuid;

pub struct GetProductQuery(pub Uuid);
impl Request<Option<Product>> for GetProductQuery {}

pub struct GetProductHandler(pub BoxedProductService);
impl RequestHandler<GetProductQuery, Option<Product>> for GetProductHandler {
    fn handle(&mut self, request: GetProductQuery) -> Option<Product> {
        self.0.lock().unwrap().get(request.0)
    }
}
