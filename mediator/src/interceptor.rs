
// pub enum Ordering {
//     Before,
//     After,
// }

#[cfg(feature = "streams")]
use {
    crate::futures::Stream,
    crate::StreamRequest
};

pub trait Interceptor<Req, Res> {
    fn handle(&mut self, req: Req, next: Box<dyn FnOnce(Req) -> Res>) -> Res;
}

#[cfg(feature = "async")]
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait AsyncInterceptor<Req, Res> where Req: Send, {
    async fn handle(&mut self, req: Req, next: Box<dyn FnOnce(Req) -> Res>) -> Res;
}

#[cfg(feature = "streams")]
pub trait StreamInterceptor {
    /// Type of the request handled by this handler.
    type Request: StreamRequest<Stream = Self::Stream, Item = Self::Item>;

    /// Type of the stream produced by this request.
    type Stream: Stream<Item = Self::Item>;

    /// Type of the item produced by the stream.
    type Item;

    fn handle_stream(&mut self, req: Self::Request, next: Box<dyn FnOnce(Self::Request) -> Self::Stream>) -> Self::Stream;
}

#[cfg(all(feature = "async", feature = "streams"))]
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait AsyncStreamInterceptor {
    /// Type of the request handled by this handler.
    type Request: StreamRequest<Stream = Self::Stream, Item = Self::Item>;

    /// Type of the stream produced by this request.
    type Stream: Stream<Item = Self::Item>;

    /// Type of the item produced by the stream.
    type Item;

    async fn handle_stream(&mut self, req: Self::Request, next: Box<dyn FnOnce(Self::Request) -> Self::Stream>) -> Self::Stream;
}

