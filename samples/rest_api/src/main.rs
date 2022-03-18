mod commands;
mod endpoints;
mod models;
mod queries;
mod services;
mod events;

use std::sync::{Arc, Mutex};
use crate::services::redis_service::{RedisService, SharedRedisService};
use actix_web::{web, App, HttpServer, middleware};
use actix_web::middleware::TrailingSlash;
use actix_web::web::Data;
use serde::de::DeserializeOwned;
use serde::Serialize;
use mediator::DefaultMediator;
use crate::models::product::Product;

pub type SharedMediator = Arc<Mutex<DefaultMediator>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));

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
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

fn create_mediator_service(redis: &SharedRedisService<Product>) -> SharedMediator {
    let mut mediator = DefaultMediator::new();
    mediator.add_handler(queries::get_product::GetProductRequestHandler(redis.clone()));
    mediator.add_handler(queries::get_all_products::GetAllProductsRequestHandler(redis.clone()));
    mediator.add_handler(commands::add_product::AddProductRequestHandler(redis.clone()));
    mediator.add_handler(commands::update_product::UpdateProductRequestHandler(redis.clone()));
    mediator.add_handler(commands::delete_product::DeleteProductRequestHandler(redis.clone()));

    Arc::new(Mutex::new(mediator))
}

fn create_redis_service<V>(base_key: &str) -> SharedRedisService<V> where V: Serialize + DeserializeOwned {
    let client = redis::Client::open("redis://127.0.0.1/").expect("Failed to connect to Redis");
    let service = RedisService::new(client, base_key.to_owned());
    Arc::new(Mutex::new(service))
}
