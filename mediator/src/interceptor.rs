use crate::futures::BoxFuture;

#[cfg(feature = "streams")]
use {crate::futures::Stream, crate::StreamRequest};

// pub enum Ordering {
//     Before,
//     After,
// }

/// Provides a way to capture the requests.
pub trait Interceptor<Req, Res> {
    /// Handles the next request.
    fn handle(&mut self, req: Req, next: Box<dyn FnOnce(Req) -> Res>) -> Res;
}

/// Provides a way to capture stream requests.
#[cfg(feature = "streams")]
pub trait StreamInterceptor {
    /// Type of the request handled by this interceptor.
    type Request: StreamRequest<Stream = Self::Stream, Item = Self::Item>;

    /// Type of the stream produced by this interceptor.
    type Stream: Stream<Item = Self::Item>;

    /// Type of the item produced by the stream.
    type Item;

    /// Handles the next stream request.
    fn handle_stream(
        &mut self,
        req: Self::Request,
        next: Box<dyn FnOnce(Self::Request) -> Self::Stream>,
    ) -> Self::Stream;
}

/// Provides an asynchronous way to capture the requests.
#[cfg(feature = "async")]
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait AsyncInterceptor<Req, Res>
where
    Req: Send,
{
    /// Handles the next request.
    async fn handle(
        &mut self,
        req: Req,
        next: Box<dyn FnOnce(Req) -> BoxFuture<'static, Res> + Send>,
    ) -> Res;
}

/// Provides an asynchronous way to capture stream requests.
#[cfg(all(feature = "async", feature = "streams"))]
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait AsyncStreamInterceptor {
    /// Type of the request handled by this interceptor.
    type Request: StreamRequest<Stream = Self::Stream, Item = Self::Item>;

    /// Type of the stream produced by this interceptor.
    type Stream: Stream<Item = Self::Item>;

    /// Type of the item produced by the stream.
    type Item;

    /// Handles the next stream request.
    async fn handle_stream(
        &mut self,
        req: Self::Request,
        next: Box<dyn FnOnce(Self::Request) -> BoxFuture<'static, Self::Stream> + Send>,
    ) -> Self::Stream;
}
