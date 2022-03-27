use crate::events::ProductAddedEvent;
use crate::models::product::Product;
use crate::services::redis_service::SharedRedisService;
use mediator::{AsyncMediator, AsyncRequestHandler, DefaultAsyncMediator, Request};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProductCommand {
    pub name: String,
    pub price: f32,
}

impl Request<Product> for AddProductCommand {}

pub struct AddProductRequestHandler(pub SharedRedisService<Product>, pub DefaultAsyncMediator);
#[mediator::async_trait]
impl AsyncRequestHandler<AddProductCommand, Product> for AddProductRequestHandler {
    async fn handle(&mut self, command: AddProductCommand) -> Product {
        let product = Product {
            id: Uuid::new_v4(),
            name: command.name,
            price: command.price,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let mut lock = self.0.lock().await;

        lock.set(product.id.to_string(), product.clone())
            .await
            .expect("Could not set product in redis");

        self.1
            .publish(ProductAddedEvent(product.clone()))
            .await
            .expect("Could not publish event");

        product
    }
}
