use crate::events::ProductUpdatedEvent;
use crate::Product;
use crate::SharedRedisService;
use mediator::{AsyncMediator, AsyncRequestHandler, DefaultAsyncMediator, Request};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProductCommand {
    pub id: Uuid,
    pub name: Option<String>,
    pub price: Option<f32>,
}

impl Request<Option<Product>> for UpdateProductCommand {}

pub struct UpdateProductRequestHandler(pub SharedRedisService<Product>, pub DefaultAsyncMediator);

#[mediator::async_trait]
impl AsyncRequestHandler<UpdateProductCommand, Option<Product>> for UpdateProductRequestHandler {
    async fn handle(&mut self, command: UpdateProductCommand) -> Option<Product> {
        let mut redis = self.0.lock().await;

        let id = command.id.to_string();
        let mut product = redis.get(&id).await.expect("Could not get the product")?;
        let will_update = command.name.is_some() || command.price.is_some();
        product.name = command.name.unwrap_or(product.name);
        product.price = command.price.unwrap_or(product.price);

        if will_update {
            product.updated_at = chrono::Utc::now();
        }

        redis
            .set(&id, product.clone())
            .await
            .expect("Could not set the product");

        self.1
            .publish(ProductUpdatedEvent(product.clone()))
            .await
            .expect("Could not publish the event");

        Some(product)
    }
}
