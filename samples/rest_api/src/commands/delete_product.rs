use crate::models::Product;
use crate::BoxedProductService;
use mediator::{Request, RequestHandler};
use uuid::Uuid;

pub struct DeleteProductCommand(pub Uuid);
impl Request<Option<Product>> for DeleteProductCommand {}

pub struct DeleteProductHandler(pub BoxedProductService);
impl RequestHandler<DeleteProductCommand, Option<Product>> for DeleteProductHandler {
    fn handle(&mut self, request: DeleteProductCommand) -> Option<Product> {
        self.0.lock().unwrap().delete(request.0)
    }
}
