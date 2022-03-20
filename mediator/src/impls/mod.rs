/// Provides a default implementation of the `Mediator` trait.
mod mediator_impl;
pub use mediator_impl::*;

/// Provides an async implementation of the `Mediator` trait.
#[cfg(feature = "async")]
mod async_mediator_impl;

#[cfg(feature = "async")]
pub use async_mediator_impl::*;
