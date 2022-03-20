use crate::error::{Error, ErrorKind};
use crate::{
    AsyncEventHandler, AsyncMediator, AsyncRequestHandler, AsyncStreamRequestHandler, Event,
    Request,
};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

#[cfg(feature = "streams")]
use crate::{futures::Stream, StreamRequest};

#[cfg(feature = "streams")]
type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + Send + 'a>>;

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
        H: AsyncRequestHandler<Req, Res> + Sync + Send + 'static,
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

// A wrapper around the stream handler to handle the request.
// To provide type safety without unsafe code we box all: the function, the params and the result.
#[derive(Clone)]
struct EventHandlerWrapper {
    #[allow(clippy::type_complexity)]
    handler: Arc<AsyncMutex<dyn FnMut(Box<dyn Any + Send>) -> BoxFuture<'static, ()> + Send>>,
}

impl EventHandlerWrapper {
    pub fn new<E, H>(handler: H) -> Self
    where
        E: Event + Send + 'static,
        H: AsyncEventHandler<E> + Sync + Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |event: Box<dyn Any + Send>| -> BoxFuture<'static, ()> {
            let handler = handler.clone();
            let event = *event.downcast::<E>().unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                req_handler.handle(event).await;
            };

            Box::pin(res)
        };

        EventHandlerWrapper {
            handler: Arc::new(AsyncMutex::new(f)),
        }
    }

    pub fn from_fn<E, H, F>(handler: H) -> Self
    where
        E: Event + Send + 'static,
        H: FnMut(E) -> F + Send + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |event: Box<dyn Any + Send>| -> BoxFuture<'static, ()> {
            let handler = handler.clone();
            let event = *event.downcast::<E>().unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                (req_handler)(event).await;
            };

            Box::pin(res)
        };

        EventHandlerWrapper {
            handler: Arc::new(AsyncMutex::new(f)),
        }
    }

    pub async fn handle<E>(&mut self, event: E)
    where
        E: Event + Sync + Send + 'static,
    {
        let event = Box::new(event);
        let mut lock = self.handler.lock().await;
        (lock)(event).await;
    }
}

// A wrapper around the request handler to handle the request and return the result.
// To provide type safety without unsafe code we box all: the function, the params and the result.
#[derive(Clone)]
#[cfg(feature = "streams")]
struct StreamRequestHandlerWrapper {
    #[allow(clippy::type_complexity)]
    handler: Arc<
        AsyncMutex<(dyn FnMut(Box<dyn Any + Send>) -> BoxFuture<'static, Box<dyn Any>> + Send)>,
    >,
}

#[cfg(feature = "streams")]
impl StreamRequestHandlerWrapper {
    pub fn new<Req, S, T, H>(handler: H) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + Send + 'static,
        H: AsyncStreamRequestHandler<Request = Req, Stream = S, Item = T> + Send + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |req: Box<dyn Any + Send>| -> BoxFuture<'static, Box<dyn Any>> {
            let handler = handler.clone();
            let req = *req.downcast::<Req>().unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                let res = req_handler.handle_stream(req).await;
                let box_res: Box<dyn Any> = Box::new(res);
                box_res
            };

            Box::pin(res)
        };

        StreamRequestHandlerWrapper {
            handler: Arc::new(AsyncMutex::new(f)),
        }
    }

    pub fn from_fn<Req, S, T, H, F>(handler: H) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + Send + 'static,
        H: FnMut(Req) -> F + Send + 'static,
        F: Future<Output = S> + Send + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |req: Box<dyn Any + Send>| -> BoxFuture<'static, Box<dyn Any>> {
            let handler = handler.clone();
            let req = *req.downcast::<Req>().unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                let res = (req_handler)(req).await;
                let box_res: Box<dyn Any> = Box::new(res);
                box_res
            };

            Box::pin(res)
        };

        StreamRequestHandlerWrapper {
            handler: Arc::new(AsyncMutex::new(f)),
        }
    }

    pub async fn handle<Req, S, T>(&mut self, req: Req) -> Option<S>
    where
        Req: StreamRequest<Stream = S, Item = T> + Sync + Send + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
    {
        let req = Box::new(req);
        let mut lock = self.handler.lock().await;
        let res = (lock)(req).await;
        res.downcast::<S>().map(|res| *res).ok()
    }
}

/// A default implementation for the [AsyncMediator] trait.
#[derive(Clone)]
pub struct DefaultAsyncMediator {
    request_handlers: SharedHandler<RequestHandlerWrapper>,
    event_handlers: SharedHandler<Vec<EventHandlerWrapper>>,

    #[cfg(feature = "streams")]
    stream_request_handlers: SharedHandler<StreamRequestHandlerWrapper>,
}

impl DefaultAsyncMediator {
    /// Gets a [DefaultAsyncMediator] builder.
    pub fn builder() -> DefaultAsyncMediatorBuilder {
        DefaultAsyncMediatorBuilder::new()
    }
}

// SAFETY: the `request_handlers` and `event_handlers` are wrapped in Arc and Mutex.
unsafe impl Send for DefaultAsyncMediator {}

// SAFETY: the `request_handlers` and `event_handlers` are wrapped in Arc and Mutex.
unsafe impl Sync for DefaultAsyncMediator {}

#[async_trait::async_trait]
impl AsyncMediator for DefaultAsyncMediator {
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

    async fn publish<E>(&mut self, event: E) -> crate::Result<()>
    where
        E: Event + Sync + Send + 'static,
    {
        let type_id = TypeId::of::<E>();
        let mut handlers_lock = self
            .event_handlers
            .try_lock()
            .expect("Event handlers are locked");

        if let Some(handlers) = handlers_lock.get_mut(&type_id).cloned() {
            // Drop the lock to avoid deadlocks
            drop(handlers_lock);

            for mut handler in handlers {
                handler.handle(event.clone()).await;
            }
        }

        Ok(())
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

/// A builder for the [DefaultAsyncMediator].
pub struct DefaultAsyncMediatorBuilder {
    inner: DefaultAsyncMediator,
}

impl DefaultAsyncMediatorBuilder {
    pub fn new() -> Self {
        Self {
            inner: DefaultAsyncMediator {
                request_handlers: SharedHandler::default(),
                event_handlers: SharedHandler::default(),

                #[cfg(feature = "streams")]
                stream_request_handlers: SharedHandler::default(),
            },
        }
    }

    pub fn add_handler<Req, Res, H>(self, handler: H) -> Self
    where
        Req: Request<Res> + Send + 'static,
        Res: Send + 'static,
        H: AsyncRequestHandler<Req, Res> + Sync + Send + 'static,
    {
        let mediator = self.inner.clone();

        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let handler = RequestHandlerWrapper::new(handler);
                let mut handlers = mediator.request_handlers.lock().await;
                handlers.insert(TypeId::of::<Req>(), handler);
            });
        });

        self
    }

    pub fn add_handler_fn<Req, Res, H, F>(self, handler: H) -> Self
    where
        Res: Send + 'static,
        Req: Request<Res> + Send + 'static,
        F: Future<Output = Res> + Send + 'static,
        H: FnMut(Req) -> F + Send + 'static,
    {
        let mediator = self.inner.clone();

        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let handler = RequestHandlerWrapper::from_fn(handler);
                let mut handlers = mediator.request_handlers.lock().await;
                handlers.insert(TypeId::of::<Req>(), handler);
            });
        });
        self
    }

    pub fn subscribe<E, H>(self, handler: H) -> Self
    where
        E: Event + Send + 'static,
        H: AsyncEventHandler<E> + Sync + Send + 'static,
    {
        let mediator = self.inner.clone();

        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let handler = EventHandlerWrapper::new(handler);
                let mut handlers = mediator.event_handlers.lock().await;
                let event_handlers = handlers.entry(TypeId::of::<E>()).or_insert_with(Vec::new);
                event_handlers.push(handler);
            });
        });

        self
    }

    pub fn subscribe_fn<E, H, F>(self, handler: H) -> Self
    where
        E: Event + Send + 'static,
        F: Future<Output = ()> + Send + 'static,
        H: FnMut(E) -> F + Send + 'static,
    {
        let mediator = self.inner.clone();

        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let handler = EventHandlerWrapper::from_fn(handler);
                let mut handlers = mediator.event_handlers.lock().await;
                let event_handlers = handlers.entry(TypeId::of::<E>()).or_insert_with(Vec::new);
                event_handlers.push(handler);
            });
        });

        self
    }

    #[cfg(feature = "streams")]
    pub fn add_stream_handler<Req, S, T, H>(self, handler: H) -> Self
        where
            Req: StreamRequest<Stream = S, Item = T> + Send + 'static,
            H: AsyncStreamRequestHandler<Request = Req, Stream = S, Item = T> + Send + 'static,
            S: Stream<Item = T> + 'static,
            T: 'static,
    {
        let mediator = self.inner.clone();

        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let handler = StreamRequestHandlerWrapper::new(handler);
                let mut handlers = mediator.stream_request_handlers.lock().await;
                handlers.insert(TypeId::of::<Req>(), handler);
            });
        });

        self
    }

    #[cfg(feature = "streams")]
    pub fn add_stream_handler_fn<Req, S, T, H, F>(self, handler: H) -> Self
        where
            Req: StreamRequest<Stream = S, Item = T> + Send + 'static,
            F: Future<Output = S> + Send + 'static,
            H: FnMut(Req) -> F + Send + 'static,
            S: Stream<Item = T> + 'static,
            T: 'static,
    {
        let mediator = self.inner.clone();

        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let handler = StreamRequestHandlerWrapper::from_fn(handler);
                let mut handlers = mediator.stream_request_handlers.lock().await;
                handlers.insert(TypeId::of::<Req>(), handler);
            });
        });

        self
    }

    pub fn build(self) -> DefaultAsyncMediator {
        self.inner
    }
}

impl Default for DefaultAsyncMediatorBuilder {
    fn default() -> Self {
        Self::new()
    }
}
