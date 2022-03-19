use crate::futures::Stream;
use crate::StreamRequest;

/// Handles a request and returns a stream of responses.
#[async_trait::async_trait]
pub trait StreamRequestHandler {
    /// Type of the request handled by this handler.
    type Request: StreamRequest<Stream = Self::Stream, Item = Self::Item>;

    /// Type of the stream produced by this request.
    type Stream: Stream<Item = Self::Item>;

    /// Type of the item produced by the stream.
    type Item;

    /// Handles a request and returns a stream of responses.
    async fn handle_stream(&mut self, req: Self::Request) -> Self::Stream;
}