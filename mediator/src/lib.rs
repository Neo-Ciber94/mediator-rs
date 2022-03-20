#![allow(clippy::needless_doctest_main)]
//! # mediator-rs
//!
//! ![https://github.com/Neo-Ciber94/mediator-rs/blob/main/LICENSE](https://img.shields.io/badge/license-MIT-blue)
//! ![https://docs.rs/mediator/latest/mediator/](https://img.shields.io/badge/docs-passing-brightgreen)
//! ![https://crates.io/crates/mediator](https://img.shields.io/badge/crates.io-v0.1-brightgreen)
//!
//! An implementation of the Mediator pattern in Rust
//! inspired in C# [MediatR](https://github.com/jbogard/MediatR).
//!
//! ## Mediator Pattern
//! https://en.wikipedia.org/wiki/Mediator_pattern
//!
//! ## Usage
//! ```toml
//! [dependencies]
//! mediator = "0.1"
//! ```
//!
//! ## Example
//! ```rust
//! use mediator::{DefaultMediator, Mediator, Request, Event, RequestHandler, EventHandler};
//!
//! #[derive(Clone, Debug)]
//! enum Op {
//!  Add(f32, f32),
//!  Sub(f32, f32),
//!  Mul(f32, f32),
//!  Div(f32, f32),
//! }
//!
//! struct MathRequest(Op);
//! impl Request<Option<f32>> for MathRequest {}
//!
//! #[derive(Clone, Debug)]
//! struct MathEvent(Op, Option<f32>);
//! impl Event for MathEvent {}
//!
//! struct MathRequestHandler(DefaultMediator);
//! impl RequestHandler<MathRequest, Option<f32>> for MathRequestHandler {
//!     fn handle(&mut self, req: MathRequest) -> Option<f32> {
//!         let result = match req.0 {
//!             Op::Add(a, b) => Some(a + b),
//!             Op::Sub(a, b) => Some(a - b),
//!             Op::Mul(a, b) => Some(a * b),
//!             Op::Div(a, b) => {
//!                 if b == 0.0 { None } else { Some(a / b) }
//!             }
//!         };
//!
//!         self.0.publish(MathEvent(req.0, result));
//!         result
//!     }
//! }
//!
//! fn main() {
//!     let mut mediator = DefaultMediator::builder()
//!         .add_handler_deferred(|m| MathRequestHandler(m))
//!         .subscribe_fn(|event: MathEvent| {
//!            println!("{:?}", event);
//!          })
//!         .build();
//! }
//! ```

/// A convenient result type.
pub type Result<T> = std::result::Result<T, error::Error>;

/// Module for the mediator request-response.
mod request;
pub use request::*;

/// Module for the mediator events.
mod event;
pub use event::*;

/// Module for the mediator.
mod mediator;
pub use crate::mediator::*;

/// Module for the errors.
pub mod error;

/// Provides default implementations.
#[cfg(feature = "impls")]
mod impls;

#[cfg(feature = "impls")]
pub use impls::*;

/// Module for streams.
#[cfg(feature = "streams")]
mod stream;

#[cfg(feature = "streams")]
pub use stream::*;

/// Re-exports for futures types.
#[cfg(feature = "async")]
pub mod futures;
