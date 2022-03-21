/// Provides a default implementation of the `Mediator` trait.
pub(crate) mod mediator_impl;

/// Provides an async implementation of the `Mediator` trait.
#[cfg(feature = "async")]
pub(crate) mod async_mediator_impl;
