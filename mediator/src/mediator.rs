use crate::request::Request;
use crate::Event;

/// A mediator is a central hub for communication between components.
pub trait Mediator {
    /// Sends a request to the mediator.
    fn send<Req, Res>(&mut self, req: Req) -> crate::Result<Res>
    where
        Res: 'static,
        Req: Request<Res> + 'static;

    /// Publish an event to the mediator.
    fn publish<E>(&mut self, event: E) -> crate::Result<()>
    where
        E: Event + 'static;
}


