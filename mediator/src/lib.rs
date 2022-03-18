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
#[cfg(feature="impls")]
mod impls;

#[cfg(feature="impls")]
pub use impls::*;


