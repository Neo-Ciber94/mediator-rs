#![allow(dead_code)]

mod commands;
mod events;
mod models;
mod queries;
mod service;

use crate::commands::add_product::{AddProductCommand, AddProductHandler};
use crate::commands::delete_product::DeleteProductHandler;
use crate::commands::update_product::UpdateProductHandler;
use crate::events::ProductAddedEvent;
use crate::queries::get_all_products::GetAllProductsHandler;
use crate::queries::get_product::{GetProductHandler, GetProductQuery};
use crate::service::ProductService;
use mediator::{DefaultMediator, Mediator};
use std::sync::{Arc, Mutex};

pub type BoxedProductService = Arc<Mutex<ProductService>>;

fn main() {
    let product_service = Arc::new(Mutex::new(ProductService::new()));
    let mut mediator = DefaultMediator::new();
    init_mediator(&mut mediator, &product_service);

    mediator.send(AddProductCommand("Apple", 0.65)).unwrap();
    mediator.send(AddProductCommand("Pizza", 12.0)).unwrap();
    mediator.send(AddProductCommand("Coffee", 2.0)).unwrap();
    mediator.send(AddProductCommand("Milk", 1.5)).unwrap();

    let added = mediator.send(AddProductCommand("Bread", 1.0)).unwrap();

    let result = mediator.send(GetProductQuery(added.id())).unwrap();
    println!("{:#?}", result);

    println!("{:#?}", product_service.lock().unwrap().get_all());
}

fn init_mediator(mediator: &mut DefaultMediator, product_service: &BoxedProductService) {
    mediator.add_handler(GetProductHandler(product_service.clone()));
    mediator.add_handler(GetAllProductsHandler(product_service.clone()));
    mediator.add_handler(AddProductHandler(product_service.clone(), mediator.clone()));
    mediator.add_handler(UpdateProductHandler(product_service.clone()));
    mediator.add_handler(DeleteProductHandler(product_service.clone()));

    mediator.subscribe_fn(|event: ProductAddedEvent| {
        println!("Product added: {:?}", event);
    });
}
