use uuid::Uuid;
use mediator::{Request, RequestHandler};
use crate::BoxedProductService;
use crate::models::Product;

pub struct GetProductQuery(pub Uuid);
impl Request<Option<Product>> for GetProductQuery {}

pub struct GetProductHandler(pub BoxedProductService);
impl RequestHandler<GetProductQuery, Option<Product>> for GetProductHandler {
    fn handle(&mut self, request: GetProductQuery) -> Option<Product> {
        self.0.lock().unwrap().get(request.0)
    }
}
