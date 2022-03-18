mod commands;
mod endpoints;
mod events;
mod models;
mod queries;
mod services;

use crate::models::product::Product;
use crate::services::redis_service::{RedisService, SharedRedisService};
use actix_web::middleware::TrailingSlash;
use actix_web::web::Data;
use actix_web::{middleware, web, App, HttpServer};
use mediator::DefaultMediator;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::{Arc, Mutex};

pub type SharedMediator = Arc<Mutex<DefaultMediator>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));

    let port = std::env::var("PORT")
        .map(|port| port.parse::<u16>().ok())
        .ok()
        .flatten()
        .unwrap_or(8080);

    let redis_service = create_redis_service::<Product>("products");
    let mediator = create_mediator_service(&redis_service);

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::NormalizePath::new(TrailingSlash::Always))
            .wrap(middleware::Logger::default())
            .app_data(Data::new(mediator.clone()))
            .app_data(Data::new(redis_service.clone()))
            .service(
                web::scope("/api/products")
                    .service(endpoints::products::create)
                    .service(endpoints::products::update)
                    .service(endpoints::products::delete)
                    .service(endpoints::products::get)
                    .service(endpoints::products::get_all),
            )
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

fn create_mediator_service(redis: &SharedRedisService<Product>) -> SharedMediator {
    use commands::*;
    use events::*;
    use queries::*;

    let mut mediator = DefaultMediator::new();
    mediator.add_handler(get_product::GetProductRequestHandler(redis.clone()));
    mediator.add_handler(get_all_products::GetAllProductsRequestHandler(
        redis.clone(),
    ));
    mediator.add_handler(add_product::AddProductRequestHandler(
        redis.clone(),
        mediator.clone(),
    ));
    mediator.add_handler(update_product::UpdateProductRequestHandler(
        redis.clone(),
        mediator.clone(),
    ));
    mediator.add_handler(delete_product::DeleteProductRequestHandler(
        redis.clone(),
        mediator.clone(),
    ));

    // Events
    mediator.subscribe_fn(|event: ProductAddedEvent| {
        log::info!("Added: {} - {}", event.0.name, event.0.id);
    });

    mediator.subscribe_fn(|event: ProductUpdatedEvent| {
        log::info!("Updated: {} - {}", event.0.name, event.0.id);
    });

    mediator.subscribe_fn(|event: ProductDeletedEvent| {
        log::info!("Deleted: {} - {}", event.0.name, event.0.id);
    });

    Arc::new(Mutex::new(mediator))
}

fn create_redis_service<V>(base_key: &str) -> SharedRedisService<V>
where
    V: Serialize + DeserializeOwned,
{
    let client = redis::Client::open("redis://127.0.0.1/").expect("Failed to connect to Redis");
    let service = RedisService::new(client, base_key.to_owned());
    Arc::new(Mutex::new(service))
}
