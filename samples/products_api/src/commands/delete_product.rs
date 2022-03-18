use uuid::Uuid;
use mediator::{DefaultMediator, Mediator, Request, RequestHandler};
use crate::{Product, SharedRedisService};
use crate::events::ProductDeletedEvent;

pub struct DeleteProductCommand(pub Uuid);
impl Request<Option<Product>> for DeleteProductCommand {}

pub struct DeleteProductRequestHandler<M: Mediator>(pub SharedRedisService<Product>, pub M);
impl<M: Mediator> RequestHandler<DeleteProductCommand, Option<Product>> for DeleteProductRequestHandler<M> {
    fn handle(&mut self, request: DeleteProductCommand) -> Option<Product> {
        let result = self.0.try_lock()
            .expect("Could not lock the redis service")
            .delete(request.0.to_string())
            .expect("Could not delete the product");

        if let Some(deleted) = result.clone() {
            self.1.publish(ProductDeletedEvent(deleted));
        }

        result
    }
}