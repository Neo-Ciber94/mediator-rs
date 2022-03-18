use actix_web::{get, post, Responder, HttpResponse, web};
use actix_web::web::{Data, Json};
use uuid::Uuid;
use mediator::Mediator;
use crate::SharedMediator;
use crate::commands::add_product::AddProductCommand;
use crate::queries::get_all_products::GetAllProductsRequest;
use crate::queries::get_product::GetProductRequest;

#[post("/")]
pub async fn create(mediator: Data<SharedMediator>, body: Json<AddProductCommand>) -> impl Responder {
    let mut mediator = mediator.try_lock().expect("Unable to lock mediator");
    let result = mediator.send(body.into_inner()).expect("Unable to send command");

    HttpResponse::Created()
        .insert_header(("Location", format!("/products/{}", result.id)))
        .json(result)
}

#[get("/{id}")]
pub async fn get(path: web::Path<Uuid>, mediator: Data<SharedMediator>) -> impl Responder {
    let uuid = path.into_inner();
    println!("uuid: {}", uuid);
    let mut mediator = mediator.try_lock().expect("Unable to lock mediator");
    let result = mediator.send(GetProductRequest(uuid)).expect("Unable to send command");

    HttpResponse::Ok().json(result)
}

#[get("/")]
pub async fn get_all(mediator: Data<SharedMediator>) -> impl Responder {
    let mut mediator = mediator.try_lock().expect("Unable to lock mediator");
    let result = mediator.send(GetAllProductsRequest).expect("Unable to send command");
    HttpResponse::Ok().json(result)
}