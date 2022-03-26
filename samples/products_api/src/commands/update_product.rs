use crate::events::ProductUpdatedEvent;
use crate::{Product, SharedRedisService};
use mediator::{DefaultMediator, Mediator, Request, RequestHandler};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProductCommand {
    pub id: Uuid,
    pub name: Option<String>,
    pub price: Option<f32>,
}

impl Request<Option<Product>> for UpdateProductCommand {}

pub struct UpdateProductRequestHandler(pub SharedRedisService<Product>, pub DefaultMediator);
impl RequestHandler<UpdateProductCommand, Option<Product>> for UpdateProductRequestHandler {
    fn handle(&mut self, command: UpdateProductCommand) -> Option<Product> {
        let mut redis = self.0.try_lock().expect("Could not lock the redis service");

        let id = command.id.to_string();
        let mut product = redis.get(&id).expect("Could not get the product")?;
        let will_update = command.name.is_some() || command.price.is_some();
        product.name = command.name.unwrap_or(product.name);
        product.price = command.price.unwrap_or(product.price);

        if will_update {
            product.updated_at = chrono::Utc::now();
        }

        redis
            .set(&id, product.clone())
            .expect("Could not set the product");

        self.1
            .publish(ProductUpdatedEvent(product.clone()))
            .expect("Could not publish the event");

        Some(product)
    }
}
