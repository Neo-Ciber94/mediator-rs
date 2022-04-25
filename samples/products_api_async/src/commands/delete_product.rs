use crate::events::ProductDeletedEvent;
use crate::Product;
use crate::SharedRedisService;
use mediator::{AsyncMediator, AsyncRequestHandler, DefaultAsyncMediator, Request};
use uuid::Uuid;

pub struct DeleteProductCommand(pub Uuid);
impl Request<Option<Product>> for DeleteProductCommand {}

pub struct DeleteProductRequestHandler(pub SharedRedisService<Product>, pub DefaultAsyncMediator);
#[mediator::async_trait]
impl AsyncRequestHandler<DeleteProductCommand, Option<Product>> for DeleteProductRequestHandler {
    async fn handle(&mut self, request: DeleteProductCommand) -> Option<Product> {
        let mut lock = self.0.lock().await;
        let result = lock
            .delete(request.0.to_string())
            .await
            .expect("Could not delete the product");

        if let Some(deleted) = result.clone() {
            self.1
                .publish(ProductDeletedEvent(deleted))
                .await
                .expect("Could not publish the event");
        }

        result
    }
}
