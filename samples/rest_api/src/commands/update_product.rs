use crate::models::Product;
use crate::BoxedProductService;
use mediator::{Request, RequestHandler};

pub struct UpdateProductCommand(pub Product);
impl Request<Option<Product>> for UpdateProductCommand {}

pub struct UpdateProductHandler(pub BoxedProductService);
impl RequestHandler<UpdateProductCommand, Option<Product>> for UpdateProductHandler {
    fn handle(&mut self, request: UpdateProductCommand) -> Option<Product> {
        self.0.lock().unwrap().update(request.0)
    }
}
