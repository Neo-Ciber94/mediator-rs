/// Represents a request to the mediator.
pub trait Request<Res> {}

/// Handles a request from the mediator.
pub trait RequestHandler<Req, Res>
where
    Req: Request<Res>,
{
    /// Handle a request and returns the response.
    fn handle(&mut self, req: Req) -> Res;
}

/// Handles an async request from the mediator.
#[cfg(feature = "async")]
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait AsyncRequestHandler<Req, Res>
where
    Req: Request<Res> + Send,
{
    /// Handle a request and returns the response.
    async fn handle(&mut self, req: Req) -> Res;
}
