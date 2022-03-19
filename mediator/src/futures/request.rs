use crate::Request;

/// Handles a request to the mediator.
#[async_trait::async_trait]
pub trait RequestHandler<Req, Res>
where
    Req: Request<Res> + Send,
{
    /// Handle a request and returns the response.
    async fn handle(&mut self, req: Req) -> Res;
}
