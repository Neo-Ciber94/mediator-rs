use tokio_stream::Stream;

/// Represents a request that produces a stream of responses to the mediator.
pub trait StreamRequest {
    /// Type of the stream produced by this request.
    type Stream: Stream<Item = Self::Item>;

    /// Type of the item produced by the stream.
    type Item;
}

/// Handles a request and returns a stream of responses.
pub trait StreamRequestHandler {
    /// Type of the request handled by this handler.
    type Request: StreamRequest<Stream = Self::Stream, Item = Self::Item>;

    /// Type of the stream produced by this request.
    type Stream: Stream<Item = Self::Item>;

    /// Type of the item produced by the stream.
    type Item;

    /// Handles a request and returns a stream of responses.
    fn handle_stream(&mut self, req: Self::Request) -> Self::Stream;
}

/// Handles an async request and returns a stream of responses.
#[cfg(feature = "async")]
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait AsyncStreamRequestHandler {
    /// Type of the request handled by this handler.
    type Request: StreamRequest<Stream = Self::Stream, Item = Self::Item>;

    /// Type of the stream produced by this request.
    type Stream: Stream<Item = Self::Item>;

    /// Type of the item produced by the stream.
    type Item;

    /// Handles a request and returns a stream of responses.
    async fn handle_stream(&mut self, req: Self::Request) -> Self::Stream;
}