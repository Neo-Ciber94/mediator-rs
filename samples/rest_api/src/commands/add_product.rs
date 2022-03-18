use crate::models::Product;
use crate::{BoxedProductService, ProductAddedEvent};
use mediator::{Mediator, Request, RequestHandler};

pub struct AddProductCommand(pub &'static str, pub f64);
impl Request<Product> for AddProductCommand {}

pub struct AddProductHandler<M>(pub BoxedProductService, pub M);
impl<M> RequestHandler<AddProductCommand, Product> for AddProductHandler<M>
where
    M: Mediator,
{
    fn handle(&mut self, request: AddProductCommand) -> Product {
        let result = self
            .0
            .lock()
            .unwrap()
            .add(Product::new(request.0, request.1));
        self.1.publish(ProductAddedEvent(result.clone())).unwrap();
        result
    }
}
