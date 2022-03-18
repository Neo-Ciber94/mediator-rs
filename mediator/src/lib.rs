//! # mediator-rs
//! An implementation of the Mediator pattern in Rust
//! inspired in C# [MediatR](https://github.com/jbogard/MediatR/tree/master/src/MediatR).
//!
//! ## Mediator Pattern
//! https://en.wikipedia.org/wiki/Mediator_pattern
//!
//! ## Example
//! ```rust
//! use std::sync::{Arc, Mutex};
//! use mediator::{Mediator, DefaultMediator, Event, Request, RequestHandler};
//!
//! #[derive(Debug, Clone, Eq, PartialEq)]
//! struct Product(String);
//!
//! struct ProductService(Vec<Product>, DefaultMediator);
//!
//! // Events
//! #[derive(Clone)]
//! struct ProductAddedEvent(Product);
//! impl Event for ProductAddedEvent {}
//!
//! #[derive(Clone)]
//! struct ProductDeletedEvent(Product);
//! impl Event for ProductDeletedEvent {}
//!
//! // Requests
//! type SharedProductService = Arc<Mutex<ProductService>>;
//!
//! struct AddProductRequest(String);
//! impl Request<Product> for AddProductRequest {}
//!
//! struct AddProductRequestHandler(SharedProductService, DefaultMediator);
//! impl RequestHandler<AddProductRequest, Product> for AddProductRequestHandler {
//!     fn handle(&mut self, request: AddProductRequest) -> Product {
//!         let mut product_service = self.0.lock().unwrap();
//!         let product = Product(request.0);
//!         product_service.0.push(product.clone());
//!         self.1.publish(ProductAddedEvent(product.clone()));
//!         product
//!     }
//! }
//!
//! struct DeleteProductRequest(String);
//! impl Request<Option<Product>> for DeleteProductRequest {}
//!
//! struct DeleteProductRequestHandler(SharedProductService, DefaultMediator);
//! impl RequestHandler<DeleteProductRequest, Option<Product>> for DeleteProductRequestHandler {
//!     fn handle(&mut self, req: DeleteProductRequest) -> Option<Product> {
//!         let mut product_service = self.0.lock().unwrap();
//!
//!         if let Some(index) = product_service.0.iter().position(|p| p.0 == req.0) {
//!             let removed = product_service.0.remove(index);
//!             self.1.publish(ProductDeletedEvent(removed.clone()));
//!             Some(removed)
//!         }
//!         else {
//!            None
//!         }
//!     }
//! }
//!
//! let mut mediator = DefaultMediator::new();
//! let mut service = Arc::new(Mutex::new(ProductService(Vec::new(), mediator.clone())));
//!
//! mediator.add_handler(AddProductRequestHandler(service.clone(), mediator.clone()));
//! mediator.add_handler(DeleteProductRequestHandler(service.clone(), mediator.clone()));
//!
//! mediator.subscribe_fn(|event: ProductAddedEvent| {
//!    println!("Product added: {:?}", event.0);
//! });
//!
//! mediator.subscribe_fn(|event: ProductDeletedEvent| {
//!     println!("Product deleted: {:?}", event.0);
//! });
//!
//! assert_eq!(Ok(Product("Microwave".to_owned())), mediator.send(AddProductRequest("Microwave".to_owned())));
//! assert_eq!(Ok(Product("Microwave".to_owned())), mediator.send(AddProductRequest("Microwave".to_owned())));
//! assert_eq!(Ok(Some(Product("Microwave".to_owned()))), mediator.send(DeleteProductRequest("Microwave".to_owned())));
//! ```

/// A convenient result type.
pub type Result<T> = std::result::Result<T, error::Error>;

/// Module for the mediator request-response.
mod request;
pub use request::*;

/// Module for the mediator events.
mod event;
pub use event::*;

/// Module for the errors.
mod error;
pub use error::*;

/// Module for the mediator.
mod mediator;
pub use crate::mediator::*;

/// Provides default implementations.
#[cfg(feature = "impls")]
mod impls;

#[cfg(feature = "impls")]
pub use impls::*;
