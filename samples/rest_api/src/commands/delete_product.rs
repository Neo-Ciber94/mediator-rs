use uuid::Uuid;
use mediator::{Request, RequestHandler};
use crate::BoxedProductService;
use crate::models::Product;

pub struct DeleteProductCommand(pub Uuid);
impl Request<Option<Product>> for DeleteProductCommand {}

pub struct DeleteProductHandler(pub BoxedProductService);
impl RequestHandler<DeleteProductCommand, Option<Product>> for DeleteProductHandler {
    fn handle(&mut self, request: DeleteProductCommand) -> Option<Product> {
        self.0.lock().unwrap().delete(request.0)
    }
}