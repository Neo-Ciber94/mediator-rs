use crate::error::{Error, ErrorKind};
use crate::{
    AsyncEventHandler, AsyncMediator, AsyncRequestHandler, Event, Request, StreamRequestHandler,
};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

#[cfg(feature = "streams")]
use crate::{futures::Stream, StreamRequest};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

type SharedAsyncHandler<H> = Arc<AsyncMutex<HashMap<TypeId, H>>>;

type SharedHandler<H> = Arc<Mutex<HashMap<TypeId, H>>>;

// A wrapper around the request handler to handle the request and return the result.
// To provide type safety without unsafe code we box all: the function, the params and the result.
#[derive(Clone)]
struct RequestHandlerWrapper {
    #[allow(clippy::type_complexity)]
    handler: Arc<
        AsyncMutex<(dyn FnMut(Box<dyn Any + Send>) -> BoxFuture<'static, Box<dyn Any>> + Send)>,
    >,
    is_deferred: bool,
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
            is_deferred: false,
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
            is_deferred: false,
        }
    }

    pub fn from_deferred<Req, Res, H, F>(handler: H) -> Self
    where
        Res: Send + 'static,
        Req: Request<Res> + Send + 'static,
        F: Future<Output = Res> + Send + 'static,
        H: FnMut(Req, DefaultAsyncMediator) -> F + Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |req: Box<dyn Any + Send>| -> BoxFuture<'static, Box<dyn Any>> {
            let handler = handler.clone();
            let (req, mediator) = *req.downcast::<(Req, DefaultAsyncMediator)>().unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                let res = (req_handler)(req, mediator).await;
                let box_res: Box<dyn Any> = Box::new(res);
                box_res
            };

            Box::pin(res)
        };

        RequestHandlerWrapper {
            handler: Arc::new(AsyncMutex::new(f)),
            is_deferred: true,
        }
    }

    pub async fn handle<Req, Res>(
        &mut self,
        req: Req,
        mediator: Option<DefaultAsyncMediator>,
    ) -> Option<BoxFuture<'_, Res>>
    where
        Res: Send + 'static,
        Req: Request<Res> + Send + 'static,
    {
        let mut lock = self.handler.lock().await;
        let req: Box<dyn Any + Send> = match mediator {
            Some(mediator) => Box::new((req, mediator)),
            None => Box::new(req),
        };

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
    is_deferred: bool,
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
            is_deferred: false,
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
            is_deferred: false,
        }
    }

    pub fn from_deferred<E, H, F>(handler: H) -> Self
    where
        E: Event + Send + 'static,
        H: FnMut(E, DefaultAsyncMediator) -> F + Send + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |event: Box<dyn Any + Send>| -> BoxFuture<'static, ()> {
            let handler = handler.clone();
            let (event, mediator) = *event.downcast::<(E, DefaultAsyncMediator)>().unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                (req_handler)(event, mediator).await;
            };

            Box::pin(res)
        };

        EventHandlerWrapper {
            handler: Arc::new(AsyncMutex::new(f)),
            is_deferred: true,
        }
    }

    pub async fn handle<E>(&mut self, event: E, mediator: Option<DefaultAsyncMediator>)
    where
        E: Event + Sync + Send + 'static,
    {
        let mut lock = self.handler.lock().await;
        let event: Box<dyn Any + Send> = match mediator {
            Some(mediator) => Box::new((event, mediator)),
            None => Box::new(event),
        };

        (lock)(event).await;
    }
}

// A wrapper around the stream handler to handle the request.
// To provide type safety without unsafe code we box all: the function, the params and the result.
#[derive(Clone)]
#[cfg(feature = "streams")]
struct StreamRequestHandlerWrapper {
    #[allow(clippy::type_complexity)]
    handler: Arc<Mutex<dyn FnMut(Box<dyn Any>) -> Box<dyn Any>>>,
    is_deferred: bool,
}

#[cfg(feature = "streams")]
impl StreamRequestHandlerWrapper {
    pub fn new<Req, S, T, H>(mut handler: H) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        H: StreamRequestHandler<Request = Req, Stream = S, Item = T> + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
    {
        let f = move |req: Box<dyn Any>| -> Box<dyn Any> {
            let req = *req.downcast::<Req>().unwrap();
            Box::new(handler.handle_stream(req))
        };

        StreamRequestHandlerWrapper {
            handler: Arc::new(Mutex::new(f)),
            is_deferred: false,
        }
    }

    pub fn from_fn<Req, S, T, F>(mut handler: F) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        S: Stream<Item = T> + 'static,
        F: FnMut(Req) -> S + 'static,
        T: 'static,
    {
        let f = move |req: Box<dyn Any>| -> Box<dyn Any> {
            let req = *req.downcast::<Req>().unwrap();
            Box::new(handler(req))
        };

        StreamRequestHandlerWrapper {
            handler: Arc::new(Mutex::new(f)),
            is_deferred: false,
        }
    }

    pub fn from_deferred<Req, S, T, F>(mut handler: F) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        S: Stream<Item = T> + 'static,
        F: FnMut(Req, DefaultAsyncMediator) -> S + 'static,
        T: 'static,
    {
        let f = move |req: Box<dyn Any>| -> Box<dyn Any> {
            let (req, mediator) = *req.downcast::<(Req, DefaultAsyncMediator)>().unwrap();
            Box::new(handler(req, mediator))
        };

        StreamRequestHandlerWrapper {
            handler: Arc::new(Mutex::new(f)),
            is_deferred: true,
        }
    }

    pub fn handle<Req, S, T>(
        &mut self,
        req: Req,
        mediator: Option<DefaultAsyncMediator>,
    ) -> Option<S>
    where
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
    {
        let mut handler = self.handler.lock().unwrap();
        let req: Box<dyn Any> = match mediator {
            Some(mediator) => Box::new((req, mediator)),
            None => Box::new(req),
        };

        let res = (handler)(req);
        res.downcast::<S>().map(|res| *res).ok()
    }
}

/// A default implementation for the [AsyncMediator] trait.
#[derive(Clone)]
pub struct DefaultAsyncMediator {
    request_handlers: SharedAsyncHandler<RequestHandlerWrapper>,
    event_handlers: SharedAsyncHandler<Vec<EventHandlerWrapper>>,

    #[cfg(feature = "streams")]
    stream_handlers: SharedHandler<StreamRequestHandlerWrapper>,
}

impl DefaultAsyncMediator {
    /// Gets a [DefaultAsyncMediator] builder.
    pub fn builder() -> Builder {
        Builder::new()
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

            let mediator = if handler.is_deferred {
                Some(self.clone())
            } else {
                None
            };

            if let Some(res_future) = handler.handle(req, mediator).await {
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

        // FIXME: Cloning the entire Vec may not be necessary, we could use something like Arc<Mutex<Vec<_>>>
        if let Some(handlers) = handlers_lock.get_mut(&type_id).cloned() {
            // Drop the lock to avoid deadlocks
            drop(handlers_lock);

            for mut handler in handlers {
                let mediator = if handler.is_deferred {
                    Some(self.clone())
                } else {
                    None
                };

                handler.handle(event.clone(), mediator).await;
            }
        }

        Ok(())
    }

    #[cfg(feature = "streams")]
    fn stream<Req, S, T>(&mut self, req: Req) -> crate::Result<S>
    where
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
    {
        let type_id = TypeId::of::<Req>();
        let mut handlers_lock = self
            .stream_handlers
            .try_lock()
            .expect("Stream handlers are locked");

        if let Some(mut handler) = handlers_lock.get_mut(&type_id).cloned() {
            // Drop the lock to avoid deadlocks
            drop(handlers_lock);

            let mediator = if handler.is_deferred {
                Some(self.clone())
            } else {
                None
            };

            if let Some(stream) = handler.handle(req, mediator) {
                return Ok(stream);
            }
        }

        Err(Error::from(ErrorKind::NotFound))
    }
}

/// A builder for the [DefaultAsyncMediator].
pub struct Builder {
    inner: DefaultAsyncMediator,
}

impl Builder {
    /// Constructs a new `DefaultAsyncMediatorBuilder`.
    pub fn new() -> Self {
        Self {
            inner: DefaultAsyncMediator {
                request_handlers: SharedAsyncHandler::default(),
                event_handlers: SharedAsyncHandler::default(),

                #[cfg(feature = "streams")]
                stream_handlers: SharedHandler::default(),
            },
        }
    }

    /// Registers a request handler.
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

    /// Registers a request handler from a function.
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

    /// Register a request handler using a copy of the mediator.
    pub fn add_handler_deferred<Req, Res, H, F>(self, f: F) -> Self
    where
        Req: Request<Res> + Send + 'static,
        Res: Send + 'static,
        H: AsyncRequestHandler<Req, Res> + Sync + Send + 'static,
        F: Fn(DefaultAsyncMediator) -> H,
    {
        let handler = f(self.inner.clone());
        self.add_handler(handler)
    }

    /// Registers a request handler from a function using a copy of the mediator.
    pub fn add_handler_fn_deferred<Req, Res, U, H, F>(self, f: H) -> Self
    where
        Res: Send + 'static,
        Req: Request<Res> + Send + 'static,
        F: Future<Output = Res> + Send + 'static,
        H: FnMut(Req, DefaultAsyncMediator) -> F + Send + 'static,
    {
        let mediator = self.inner.clone();

        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let handler = RequestHandlerWrapper::from_deferred(f);
                let mut handlers = mediator.request_handlers.lock().await;
                handlers.insert(TypeId::of::<Req>(), handler);
            });
        });
        self
    }

    /// Registers an event handler.
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

    /// Registers an event handler from a function.
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

    /// Registers an event handler using a copy of the mediator.
    pub fn subscribe_deferred<E, H, F>(self, f: F) -> Self
    where
        E: Event + Send + 'static,
        H: AsyncEventHandler<E> + Sync + Send + 'static,
        F: Fn(DefaultAsyncMediator) -> H,
    {
        let handler = f(self.inner.clone());
        self.subscribe(handler)
    }

    /// Registers an event handler from a function using a copy of the mediator.
    pub fn subscribe_fn_deferred<E, H, U, F>(self, f: H) -> Self
    where
        E: Event + Send + 'static,
        F: Future<Output = ()> + Send + 'static,
        H: FnMut(E, DefaultAsyncMediator) -> F + Send + 'static,
    {
        let mediator = self.inner.clone();

        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let handler = EventHandlerWrapper::from_deferred(f);
                let mut handlers = mediator.event_handlers.lock().await;
                let event_handlers = handlers.entry(TypeId::of::<E>()).or_insert_with(Vec::new);
                event_handlers.push(handler);
            });
        });

        self
    }

    /// Registers a stream handler.
    #[cfg(feature = "streams")]
    pub fn add_stream_handler<Req, S, T, H>(self, handler: H) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        H: StreamRequestHandler<Request = Req, Stream = S, Item = T> + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
    {
        let mut handlers_lock = self.inner.stream_handlers.lock().unwrap();
        handlers_lock.insert(
            TypeId::of::<Req>(),
            StreamRequestHandlerWrapper::new(handler),
        );
        drop(handlers_lock);
        self
    }

    /// Registers a stream handler from a function.
    #[cfg(feature = "streams")]
    pub fn add_stream_handler_fn<Req, S, T, F>(self, f: F) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        F: FnMut(Req) -> S + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
    {
        let mut handlers_lock = self.inner.stream_handlers.lock().unwrap();
        handlers_lock.insert(TypeId::of::<Req>(), StreamRequestHandlerWrapper::from_fn(f));
        drop(handlers_lock);
        self
    }

    /// Registers a stream handler using a copy of the mediator.
    #[cfg(feature = "streams")]
    pub fn add_stream_handler_deferred<Req, S, T, H, F>(self, f: F) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        H: StreamRequestHandler<Request = Req, Stream = S, Item = T> + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
        F: Fn(DefaultAsyncMediator) -> H,
    {
        let handler = f(self.inner.clone());
        self.add_stream_handler(handler)
    }

    /// Registers a stream handler from a function using a copy of the mediator.
    #[cfg(feature = "streams")]
    pub fn add_stream_handler_fn_deferred<Req, S, T, F>(self, f: F) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        F: FnMut(Req, DefaultAsyncMediator) -> S + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
    {
        let mut handlers_lock = self.inner.stream_handlers.lock().unwrap();
        handlers_lock.insert(
            TypeId::of::<Req>(),
            StreamRequestHandlerWrapper::from_deferred(f),
        );
        drop(handlers_lock);
        self
    }

    /// Builds the `DefaultAsyncMediator`.
    pub fn build(self) -> DefaultAsyncMediator {
        self.inner
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {}
