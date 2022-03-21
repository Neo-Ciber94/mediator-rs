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

/// Module for streams.
#[cfg(feature = "streams")]
mod streams;

#[cfg(feature = "streams")]
pub use streams::*;

/// Re-exports for futures/stream utilities.
#[cfg(feature = "async")]
pub mod futures;

// Default implementations.
#[cfg(feature = "impls")]
mod default_impls;

// Default exports
#[cfg(feature = "impls")]
pub use default_impls::mediator_impl::DefaultMediator;

#[cfg(all(feature = "impls", feature = "async"))]
pub use default_impls::async_mediator_impl::DefaultAsyncMediator;

/// Provides default implementations.
#[cfg(feature = "impls")]
pub mod impls {
    pub use crate::default_impls::mediator_impl::Builder;
    pub use crate::default_impls::mediator_impl::DefaultMediator;
}

/// Provides async default implementations.
#[cfg(all(feature = "impls", feature = "async"))]
pub mod impls_async {
    pub use crate::default_impls::async_mediator_impl::Builder;
    pub use crate::default_impls::async_mediator_impl::DefaultAsyncMediator;
}
