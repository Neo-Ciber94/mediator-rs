use crate::request::Request;
use crate::Event;

#[cfg(feature = "streams")]
use crate::StreamRequest;

#[cfg(feature = "streams")]
use tokio_stream::Stream;

/// A mediator is a central hub for communication between components.
#[async_trait::async_trait]
pub trait Mediator {
    /// Sends a request to the mediator.
    async fn send<Req, Res>(&mut self, req: Req) -> crate::Result<Res>
    where
        Res: Send + 'static,
        Req: Request<Res> + Send + 'static;

    /// Publish an event to the mediator.
    async fn publish<E>(&mut self, event: E) -> crate::Result<()>
    where
        E: Event + Send + 'static;

    /// Sends a request to the mediator and returns a stream of responses.
    #[cfg(feature = "streams")]
    async fn stream<Req, S, T>(&mut self, req: Req) -> crate::Result<S>
    where
        Req: StreamRequest<Stream = S, Item = T> + Send + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static;
}
