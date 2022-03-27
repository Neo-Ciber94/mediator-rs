use crate::events::ProductAddedEvent;
use crate::models::product::Product;
use crate::services::redis_service::SharedRedisService;
use mediator::{DefaultMediator, Mediator, Request, RequestHandler};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProductCommand {
    pub name: String,
    pub price: f32,
}

impl Request<Product> for AddProductCommand {}

pub struct AddProductRequestHandler(pub SharedRedisService<Product>, pub DefaultMediator);
impl RequestHandler<AddProductCommand, Product> for AddProductRequestHandler {
    fn handle(&mut self, command: AddProductCommand) -> Product {
        let product = Product {
            id: Uuid::new_v4(),
            name: command.name,
            price: command.price,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.0
            .lock()
            .expect("Could not lock redis service")
            .set(product.id.to_string(), product.clone())
            .expect("Could not set product in redis");

        self.1
            .publish(ProductAddedEvent(product.clone()))
            .expect("Could not publish event");

        product
    }
}
