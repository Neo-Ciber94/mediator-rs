use uuid::Uuid;
use mediator::{Request, RequestHandler};
use crate::{Product, SharedRedisService};

pub struct DeleteProductCommand(pub Uuid);
impl Request<Option<Product>> for DeleteProductCommand {}

pub struct DeleteProductRequestHandler(pub SharedRedisService<Product>);
impl RequestHandler<DeleteProductCommand, Option<Product>> for DeleteProductRequestHandler {
    fn handle(&mut self, request: DeleteProductCommand) -> Option<Product> {
        self.0.try_lock()
            .expect("Could not lock the redis service")
            .delete(request.0.to_string())
            .expect("Could not delete the product")
    }
}