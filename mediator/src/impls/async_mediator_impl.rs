use crate::error::{Error, ErrorKind};
use crate::futures::{Mediator, RequestHandler};
use crate::{Event, Request};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

#[cfg(feature = "streams")]
use crate::{futures::Stream, StreamRequest};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

type SharedHandler<H> = Arc<AsyncMutex<HashMap<TypeId, H>>>;

// A wrapper around the request handler to handle the request and return the result.
// To provide type safety without unsafe code we box all: the function, the params and the result.
#[derive(Clone)]
struct RequestHandlerWrapper {
    #[allow(clippy::type_complexity)]
    handler: Arc<
        AsyncMutex<(dyn FnMut(Box<dyn Any + Send>) -> BoxFuture<'static, Box<dyn Any>> + Send)>,
    >,
}

impl RequestHandlerWrapper {
    pub fn new<Req, Res, H>(handler: H) -> Self
    where
        Res: 'static,
        Req: Request<Res> + Send + 'static,
        H: RequestHandler<Req, Res> + Sync + Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |req: Box<dyn Any + Send>| -> BoxFuture<'static, Box<dyn Any>> {
            let handler = handler.clone();
            let req = *req.downcast::<Req>().unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                let res = req_handler.handle(req).await;
                let box_res: Box<dyn Any> = Box::new(res);
                box_res
            };

            Box::pin(res)
        };

        RequestHandlerWrapper {
            handler: Arc::new(AsyncMutex::new(f)),
        }
    }

    pub fn from_fn<Req, Res, H, F>(handler: H) -> Self
    where
        Res: Send + 'static,
        Req: Request<Res> + Send + 'static,
        F: Future<Output = Res> + Send + 'static,
        H: FnMut(Req) -> F + Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |req: Box<dyn Any + Send>| -> BoxFuture<'static, Box<dyn Any>> {
            let handler = handler.clone();
            let req = *req.downcast::<Req>().unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                let res: Res = (req_handler)(req).await;
                let box_res: Box<dyn Any> = Box::new(res);
                box_res
            };

            Box::pin(res)
        };

        RequestHandlerWrapper {
            handler: Arc::new(AsyncMutex::new(f)),
        }
    }

    pub async fn handle<Req, Res>(&mut self, req: Req) -> Option<BoxFuture<'_, Res>>
    where
        Res: Send + 'static,
        Req: Request<Res> + Send + 'static,
    {
        let req = Box::new(req);
        let mut lock = self.handler.lock().await;
        let res_future: BoxFuture<Box<dyn Any>> = (lock)(req);
        let res_box = res_future.await;

        match res_box.downcast::<Res>().map(|res| *res).ok() {
            Some(res) => Some(Box::pin(async move { res })),
            None => None,
        }
    }
}

pub struct AsyncDefaultMediator {
    request_handlers: SharedHandler<RequestHandlerWrapper>,
}

impl AsyncDefaultMediator {
    pub fn new() -> Self {
        Self {
            request_handlers: Arc::new(AsyncMutex::new(HashMap::new())),
        }
    }

    pub fn add_handler<Req, Res, H>(&mut self, handler: H)
    where
        Req: Request<Res> + Send + 'static,
        Res: Send + 'static,
        H: RequestHandler<Req, Res> + Sync + Send + 'static,
    {
        use tokio::runtime::Handle;

        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            Handle::current().block_on(async move {
                let handler = RequestHandlerWrapper::new(handler);
                let mut handlers = self.request_handlers.lock().await;
                handlers.insert(TypeId::of::<Req>(), handler);
            });
        });
    }

    pub fn add_handler_fn<Req, Res, H, F>(&mut self, handler: H)
    where
        Res: Send + 'static,
        Req: Request<Res> + Send + 'static,
        F: Future<Output = Res> + Send + 'static,
        H: FnMut(Req) -> F + Send + 'static,
    {
        use tokio::runtime::Handle;

        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            Handle::current().block_on(async move {
                let handler = RequestHandlerWrapper::from_fn(handler);
                let mut handlers = self.request_handlers.lock().await;
                handlers.insert(TypeId::of::<Req>(), handler);
            });
        });
    }
}

#[async_trait::async_trait]
impl Mediator for AsyncDefaultMediator {
    async fn send<Req, Res>(&mut self, req: Req) -> crate::Result<Res>
    where
        Res: Send + 'static,
        Req: Request<Res> + Send + 'static,
    {
        let type_id = TypeId::of::<Req>();
        let mut handlers_lock = self
            .request_handlers
            .try_lock()
            .expect("Request handlers are locked");

        if let Some(mut handler) = handlers_lock.get_mut(&type_id).cloned() {
            // Drop the lock to avoid deadlocks
            drop(handlers_lock);

            if let Some(res_future) = handler.handle(req).await {
                let res = res_future.await;
                return Ok(res);
            }
        }

        Err(Error::from(ErrorKind::NotFound))
    }

    async fn publish<E>(&mut self, _event: E) -> crate::Result<()>
    where
        E: Event + Send + 'static,
    {
        todo!()
    }

    #[cfg(feature = "streams")]
    async fn stream<Req, S, T>(&mut self, _req: Req) -> crate::Result<S>
    where
        Req: StreamRequest<Stream = S, Item = T> + Send + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
    {
        todo!()
    }
}
