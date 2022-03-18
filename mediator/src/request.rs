
/// Represents a request to the mediator.
pub trait Request<Res> {}

/// Handles a request to the mediator.
pub trait RequestHandler<Req, Res>
    where
        Req: Request<Res>,
{
    /// Handle a request and returns the response.
    fn handle(&mut self, req: Req) -> Res;
}
