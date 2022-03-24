/// A boxed stream.
pub type BoxStream<'a, T> = std::pin::Pin<Box<dyn crate::futures::Stream<Item = T> + Send + 'a>>;

/// A boxed future.
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

#[cfg(feature = "streams")]
pub use tokio_stream::*;

/// Utilities for streams.
#[cfg(feature = "streams")]
pub mod stream;
