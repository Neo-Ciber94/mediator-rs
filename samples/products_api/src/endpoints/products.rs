use crate::commands::{AddProductCommand, DeleteProductCommand, UpdateProductCommand};
use crate::queries::{GetAllProductsRequest, GetProductRequest};
use actix_web::web::{Data, Json};
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use mediator::{DefaultMediator, Mediator};
use tokio::sync::Mutex;
use uuid::Uuid;

#[post("/")]
pub async fn create(
    mediator: Data<Mutex<DefaultMediator>>,
    body: Json<AddProductCommand>,
) -> impl Responder {
    let mut mediator = mediator.lock().await;
    let result = mediator
        .send(body.into_inner())
        .expect("Unable to send command");

    HttpResponse::Created()
        .insert_header(("Location", format!("/api/products/{}", result.id)))
        .json(result)
}

#[put("/")]
pub async fn update(
    mediator: Data<Mutex<DefaultMediator>>,
    body: Json<UpdateProductCommand>,
) -> impl Responder {
    let mut mediator = mediator.lock().await;
    let result = mediator
        .send(body.into_inner())
        .expect("Unable to send command");

    match result {
        Some(product) => HttpResponse::Ok().json(product),
        None => HttpResponse::NotFound().finish(),
    }
}

#[delete("/{id}/")]
pub async fn delete(
    path: web::Path<Uuid>,
    mediator: Data<Mutex<DefaultMediator>>,
) -> impl Responder {
    let uuid = path.into_inner();
    let mut mediator = mediator.lock().await;

    let result = mediator
        .send(DeleteProductCommand(uuid))
        .expect("Unable to send command");

    match result {
        Some(product) => HttpResponse::Ok().json(product),
        None => HttpResponse::NotFound().finish(),
    }
}

#[get("/{id}/")]
pub async fn get(path: web::Path<Uuid>, mediator: Data<Mutex<DefaultMediator>>) -> impl Responder {
    let uuid = path.into_inner();
    let mut mediator = mediator.lock().await;
    let result = mediator
        .send(GetProductRequest(uuid))
        .expect("Unable to send command");

    HttpResponse::Ok().json(result)
}

#[get("/")]
pub async fn get_all(mediator: Data<Mutex<DefaultMediator>>) -> impl Responder {
    let mut mediator = mediator.lock().await;
    let result = mediator
        .send(GetAllProductsRequest)
        .expect("Unable to send command");
    HttpResponse::Ok().json(result)
}
