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

#[cfg(feature = "interceptors")]
use crate::AsyncInterceptor;

#[cfg(feature = "streams")]
use {
    crate::futures::Stream,
    crate::{AsyncStreamInterceptor, StreamRequest},
};

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

    pub fn from_fn_with<Req, Res, H, F, S>(handler: H, state: S) -> Self
    where
        S: Send + Clone + 'static,
        Res: Send + 'static,
        Req: Request<Res> + Send + 'static,
        F: Future<Output = Res> + Send + 'static,
        H: FnMut(Req, S) -> F + Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |req: Box<dyn Any + Send>| -> BoxFuture<'static, Box<dyn Any>> {
            let handler = handler.clone();
            let state = state.clone();
            let req = *req.downcast::<Req>().unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                let res: Res = (req_handler)(req, state).await;
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

    pub fn from_deferred_with<State, Req, Res, H, F>(handler: H, state: State) -> Self
    where
        State: Send + Clone + 'static,
        Res: Send + 'static,
        Req: Request<Res> + Send + 'static,
        F: Future<Output = Res> + Send + 'static,
        H: FnMut(Req, DefaultAsyncMediator, State) -> F + Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |req: Box<dyn Any + Send>| -> BoxFuture<'static, Box<dyn Any>> {
            let handler = handler.clone();
            let state = state.clone();
            let (req, mediator) = *req.downcast::<(Req, DefaultAsyncMediator)>().unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                let res = (req_handler)(req, mediator, state).await;
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

    pub fn from_fn_with<State, E, H, F>(handler: H, state: State) -> Self
    where
        State: Send + Clone + 'static,
        E: Event + Send + 'static,
        H: FnMut(E, State) -> F + Send + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |event: Box<dyn Any + Send>| -> BoxFuture<'static, ()> {
            let handler = handler.clone();
            let state = state.clone();
            let event = *event.downcast::<E>().unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                (req_handler)(event, state).await;
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

    pub fn from_deferred_with<State, E, H, F>(handler: H, state: State) -> Self
    where
        State: Send + Clone + 'static,
        E: Event + Send + 'static,
        H: FnMut(E, DefaultAsyncMediator, State) -> F + Send + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |event: Box<dyn Any + Send>| -> BoxFuture<'static, ()> {
            let handler = handler.clone();
            let state = state.clone();
            let (event, mediator) = *event.downcast::<(E, DefaultAsyncMediator)>().unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                (req_handler)(event, mediator, state).await;
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
    handler: Arc<Mutex<dyn FnMut(Box<dyn Any>) -> Box<dyn Any> + Send + Sync>>,
    is_deferred: bool,
}

#[cfg(feature = "streams")]
impl StreamRequestHandlerWrapper {
    pub fn new<Req, S, T, H>(mut handler: H) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        H: StreamRequestHandler<Request = Req, Stream = S, Item = T> + Sync + Send + 'static,
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
        F: FnMut(Req) -> S + Send + Sync + 'static,
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

    pub fn from_fn_with<State, Req, S, T, F>(mut handler: F, state: State) -> Self
    where
        State: Sync + Send + Clone + 'static,
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        S: Stream<Item = T> + 'static,
        F: FnMut(Req, State) -> S + Send + Sync + 'static,
        T: 'static,
    {
        let f = move |req: Box<dyn Any>| -> Box<dyn Any> {
            let req = *req.downcast::<Req>().unwrap();
            Box::new(handler(req, state.clone()))
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
        F: FnMut(Req, DefaultAsyncMediator) -> S + Send + Sync + 'static,
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

    pub fn from_deferred_with<State, Req, S, T, F>(mut handler: F, state: State) -> Self
    where
        State: Sync + Send + Clone + 'static,
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        S: Stream<Item = T> + 'static,
        F: FnMut(Req, DefaultAsyncMediator, State) -> S + Send + Sync + 'static,
        T: 'static,
    {
        let f = move |req: Box<dyn Any>| -> Box<dyn Any> {
            let (req, mediator) = *req.downcast::<(Req, DefaultAsyncMediator)>().unwrap();
            Box::new(handler(req, mediator, state.clone()))
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

#[cfg(feature = "interceptors")]
type NextCallback = Box<dyn Any + Send>;
type BoxAnySendFuture = BoxFuture<'static, Box<dyn Any + Send>>;
type BoxFnOnceFuture<Req, Res> = Box<dyn FnOnce(Req) -> BoxFuture<'static, Res> + Send>;

#[cfg(feature = "interceptors")]
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct InterceptorKey {
    req_ty: TypeId,
    res_ty: TypeId,
}

#[cfg(feature = "interceptors")]
impl InterceptorKey {
    pub fn of<Req: 'static, Res: 'static>() -> Self {
        InterceptorKey {
            req_ty: TypeId::of::<Req>(),
            res_ty: TypeId::of::<Res>(),
        }
    }
}

#[cfg(feature = "interceptors")]
#[derive(Clone)]
enum InterceptorWrapper {
    Handler(Arc<AsyncMutex<dyn FnMut(Box<dyn Any + Send>, NextCallback) -> BoxAnySendFuture + Send>>),

    #[cfg(feature = "streams")]
    Stream(Arc<AsyncMutex<dyn FnMut(Box<dyn Any + Send>, NextCallback) -> BoxAnySendFuture + Send>>),
}

#[cfg(feature = "interceptors")]
impl InterceptorWrapper {
    pub fn from_handler<Req, Res, H>(handler: H) -> Self
    where
        Res: Send + 'static,
        Req: Send + 'static,
        H: AsyncInterceptor<Req, Res> + Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |req: Box<dyn Any + Send>, next: NextCallback| -> BoxAnySendFuture {
            let handler = handler.clone();
            let req = *req.downcast::<Req>().unwrap();

            let next = next
                .downcast::<Box<dyn FnOnce(Req) -> BoxFuture<'static, Res> + Send>>()
                .unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                let res = req_handler.handle(req, next).await;
                Box::new(res) as Box<dyn Any + Send>
            };

            Box::pin(res)
        };

        InterceptorWrapper::Handler(Arc::new(AsyncMutex::new(f)))
    }

    pub fn from_handler_fn<Req, Res, H>(handler: H) -> Self
    where
        Req: Send + 'static,
        Res: Send + 'static,
        H: FnMut(Req, BoxFnOnceFuture<Req, Res>) -> BoxFuture<'static, Res> + Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |req: Box<dyn Any + Send>, next: NextCallback| -> BoxAnySendFuture {
            let handler = handler.clone();
            let req = *req.downcast::<Req>().unwrap();

            let next = next
                .downcast::<Box<dyn FnOnce(Req) -> BoxFuture<'static, Res> + Send>>()
                .unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                let res = req_handler(req, next).await;
                Box::new(res) as Box<dyn Any + Send>
            };

            Box::pin(res)
        };

        InterceptorWrapper::Handler(Arc::new(AsyncMutex::new(f)))
    }

    #[cfg(feature = "streams")]
    pub fn from_stream<Req, T, S, H>(handler: H) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + Send + 'static,
        H: AsyncStreamInterceptor<Request = Req, Stream = S, Item = T> + Send + 'static,
        S: Stream<Item = T> + Send + 'static,
        T: Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |req: Box<dyn Any + Send>, next: NextCallback| -> BoxAnySendFuture {
            let handler = handler.clone();
            let req = *req.downcast::<Req>().unwrap();
            let next = next
                .downcast::<Box<dyn FnOnce(Req) -> BoxFuture<'static, S> + Send>>()
                .unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                let res_fut = req_handler.handle_stream(req, next).await;
                Box::new(res_fut) as Box<dyn Any + Send>
            };
            Box::pin(res)
        };

        InterceptorWrapper::Stream(Arc::new(AsyncMutex::new(f)))
    }

    #[cfg(feature = "streams")]
    pub fn from_stream_fn<Req, T, S, H>(handler: H) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + Send + 'static,
        H: FnMut(Req, BoxFnOnceFuture<Req, S>) -> BoxFuture<'static, S> + Send + 'static,
        S: Stream<Item = T> + Send + 'static,
        T: Send + 'static,
    {
        let handler = Arc::new(AsyncMutex::new(handler));

        let f = move |req: Box<dyn Any + Send>, next: NextCallback| -> BoxAnySendFuture {
            let handler = handler.clone();
            let req = *req.downcast::<Req>().unwrap();
            let next = next
                .downcast::<Box<dyn FnOnce(Req) -> BoxFuture<'static, S> + Send>>()
                .unwrap();

            let res = async move {
                let mut req_handler = handler.lock().await;
                let res_fut = req_handler(req, next).await;
                Box::new(res_fut) as Box<dyn Any + Send>
            };
            Box::pin(res)
        };

        InterceptorWrapper::Stream(Arc::new(AsyncMutex::new(f)))
    }

    pub async fn handle<Req, Res>(
        &mut self,
        req: Req,
        next: Box<dyn FnOnce(Req) -> BoxFuture<'static, Res> + Send>,
    ) -> Option<BoxFuture<'static, Res>>
    where
        Req: Request<Res> + Send + 'static,
        Res: Send + 'static,
    {
        if let InterceptorWrapper::Handler(handler) = self {
            let mut handler = handler.lock().await;
            let req: Box<dyn Any + Send> = Box::new(req);
            let next: Box<dyn Any + Send> = Box::new(next);

            let res_fut: BoxFuture<'static, Box<dyn Any + Send>> = (handler)(req, next);
            let res = res_fut.await;

            match res.downcast::<Res>().map(|res| *res).ok() {
                Some(res) => Some(Box::pin(async move { res })),
                None => None,
            }
        } else {
            None
        }
    }

    #[cfg(feature = "streams")]
    pub async fn stream<Req, T, S>(
        &mut self,
        req: Req,
        next: Box<dyn FnOnce(Req) -> BoxFuture<'static, S> + Send>,
    ) -> Option<S>
    where
        Req: StreamRequest<Stream = S, Item = T> + Send + 'static,
        S: Stream<Item = T> + Send + 'static,
        T: Send + 'static,
    {
        if let InterceptorWrapper::Stream(handler) = self {
            let mut handler = handler.lock().await;
            let req: Box<dyn Any + Send> = Box::new(req);
            let next: Box<dyn Any + Send> = Box::new(next);

            let stream_fut: BoxFuture<'static, Box<dyn Any + Send>> = (handler)(req, next);
            let stream = stream_fut.await;

            match stream.downcast::<S>().map(|stream| *stream).ok() {
                Some(stream) => Some(stream),
                None => None,
            }
        } else {
            None
        }
    }
}

/// A default implementation for the [AsyncMediator] trait.
#[derive(Clone)]
pub struct DefaultAsyncMediator {
    request_handlers: SharedAsyncHandler<RequestHandlerWrapper>,
    event_handlers: SharedAsyncHandler<Vec<EventHandlerWrapper>>,

    #[cfg(feature = "interceptors")]
    interceptors: Arc<AsyncMutex<HashMap<InterceptorKey, Vec<InterceptorWrapper>>>>,

    #[cfg(feature = "streams")]
    stream_handlers: SharedHandler<StreamRequestHandlerWrapper>,
}

impl DefaultAsyncMediator {
    /// Gets a [DefaultAsyncMediator] builder.
    pub fn builder() -> Builder {
        Builder::new()
    }
}

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

            #[cfg(feature = "interceptors")]
            {
                //type FnFuture<Rq, Rs> = Box<dyn FnOnce(Rq) -> BoxFuture<'static, Rs> + Send>;

                let mut interceptors = self.interceptors.lock().await;

                let key = InterceptorKey::of::<Req, Res>();
                if let Some(interceptors) = interceptors.get_mut(&key).cloned() {
                    let next_handler : Box<dyn FnOnce(Req) -> BoxFuture<'static, Res> + Send> = Box::new(move |req: Req| {
                        Box::pin(async move { handler.handle(req, mediator).await.unwrap().await })
                    });

                    let handler : Box<dyn FnOnce(Req) -> BoxFuture<'static, Res> + Send> = interceptors.into_iter().fold(
                        next_handler,
                        move |next, mut interceptor| {
                            Box::new(move |req: Req| {
                                Box::pin(async move {
                                    // SAFETY: this only fail if the downcast fails,
                                    // but we already checked that the type is correct
                                    interceptor.handle(req, next).await.unwrap().await
                                })
                            })
                        },
                    );

                    let res = handler(req).await;
                    return Ok(res);
                }
            }

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
    async fn stream<Req, S, T>(&mut self, req: Req) -> crate::Result<S>
    where
        Req: StreamRequest<Stream = S, Item = T> + Send + 'static,
        S: Stream<Item = T> + Send + 'static,
        T: Send + 'static,
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

                #[cfg(feature = "interceptors")]
                interceptors: Default::default(),

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

        // FIXME: Find a way to prevent using async in the builder
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

        // FIXME: Find a way to prevent using async in the builder
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

    pub fn add_handler_fn_with<State, Req, Res, H, F>(self, state: State, handler: H) -> Self
    where
        State: Send + Clone + 'static,
        Res: Send + 'static,
        Req: Request<Res> + Send + 'static,
        F: Future<Output = Res> + Send + 'static,
        H: FnMut(Req, State) -> F + Send + 'static,
    {
        let mediator = self.inner.clone();

        // FIXME: Find a way to prevent using async in the builder
        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let handler = RequestHandlerWrapper::from_fn_with(handler, state);
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

        // FIXME: Find a way to prevent using async in the builder
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

    /// TODO
    pub fn add_handler_fn_deferred_with<State, Req, Res, U, H, F>(self, state: State, f: H) -> Self
    where
        State: Send + Clone + 'static,
        Res: Send + 'static,
        Req: Request<Res> + Send + 'static,
        F: Future<Output = Res> + Send + 'static,
        H: FnMut(Req, DefaultAsyncMediator, State) -> F + Send + 'static,
    {
        let mediator = self.inner.clone();

        // FIXME: Find a way to prevent using async in the builder
        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let handler = RequestHandlerWrapper::from_deferred_with(f, state);
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

        // FIXME: Find a way to prevent using async in the builder
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

        // FIXME: Find a way to prevent using async in the builder
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

    pub fn subscribe_fn_with<S, E, H, F>(self, state: S, handler: H) -> Self
    where
        S: Send + Clone + 'static,
        E: Event + Send + 'static,
        F: Future<Output = ()> + Send + 'static,
        H: FnMut(E, S) -> F + Send + 'static,
    {
        let mediator = self.inner.clone();

        // FIXME: Find a way to prevent using async in the builder
        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let handler = EventHandlerWrapper::from_fn_with(handler, state);
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

        // FIXME: Find a way to prevent using async in the builder
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

    pub fn subscribe_fn_deferred_with<State, E, H, U, F>(self, state: State, f: H) -> Self
    where
        State: Sync + Send + Clone + 'static,
        E: Event + Send + 'static,
        F: Future<Output = ()> + Send + 'static,
        H: FnMut(E, DefaultAsyncMediator, State) -> F + Send + 'static,
    {
        let mediator = self.inner.clone();

        // FIXME: Find a way to prevent using async in the builder
        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let handler = EventHandlerWrapper::from_deferred_with(f, state);
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
        H: StreamRequestHandler<Request = Req, Stream = S, Item = T> + Sync + Send + 'static,
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
        F: FnMut(Req) -> S + Sync + Send + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
    {
        let mut handlers_lock = self.inner.stream_handlers.lock().unwrap();
        handlers_lock.insert(TypeId::of::<Req>(), StreamRequestHandlerWrapper::from_fn(f));
        drop(handlers_lock);
        self
    }

    #[cfg(feature = "streams")]
    pub fn add_stream_handler_fn_with<State, Req, S, T, F>(self, state: State, f: F) -> Self
    where
        State: Sync + Send + Clone + 'static,
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        F: FnMut(Req, State) -> S + Sync + Send + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
    {
        let mut handlers_lock = self.inner.stream_handlers.lock().unwrap();
        handlers_lock.insert(
            TypeId::of::<Req>(),
            StreamRequestHandlerWrapper::from_fn_with(f, state),
        );
        drop(handlers_lock);
        self
    }

    /// Registers a stream handler using a copy of the mediator.
    #[cfg(feature = "streams")]
    pub fn add_stream_handler_deferred<Req, S, T, H, F>(self, f: F) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        H: StreamRequestHandler<Request = Req, Stream = S, Item = T> + Sync + Send + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
        F: Fn(DefaultAsyncMediator) -> H + Sync + Send,
    {
        let handler = f(self.inner.clone());
        self.add_stream_handler(handler)
    }

    /// Registers a stream handler from a function using a copy of the mediator.
    #[cfg(feature = "streams")]
    pub fn add_stream_handler_fn_deferred<Req, S, T, F>(self, f: F) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        F: FnMut(Req, DefaultAsyncMediator) -> S + Sync + Send + 'static,
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

    #[cfg(feature = "streams")]
    pub fn add_stream_handler_fn_deferred_with<State, Req, S, T, F>(
        self,
        state: State,
        f: F,
    ) -> Self
    where
        State: Sync + Send + Clone + 'static,
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        F: FnMut(Req, DefaultAsyncMediator, State) -> S + Sync + Send + 'static,
        S: Stream<Item = T> + 'static,
        T: 'static,
    {
        let mut handlers_lock = self.inner.stream_handlers.lock().unwrap();
        handlers_lock.insert(
            TypeId::of::<Req>(),
            StreamRequestHandlerWrapper::from_deferred_with(f, state),
        );
        drop(handlers_lock);
        self
    }

    /// Adds a request interceptor.
    #[cfg(feature = "interceptors")]
    pub fn add_interceptor<Req, Res, F, H>(self, handler: H) -> Self
    where
        Req: Send + 'static,
        Res: Send + 'static,
        H: AsyncInterceptor<Req, Res> + Send + 'static,
    {
        let req_ty = TypeId::of::<Req>();
        let res_ty = TypeId::of::<Res>();
        let key = InterceptorKey { req_ty, res_ty };

        let mediator = self.inner.clone();

        // FIXME: Find a way to prevent using async in the builder
        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let mut handlers_lock = mediator.interceptors.lock().await;
                let interceptors = handlers_lock.entry(key).or_insert(Vec::new());
                interceptors.push(InterceptorWrapper::from_handler(handler));
                drop(handlers_lock);
            });
        });

        self
    }

    /// Adds a request interceptor from a function.
    #[cfg(feature = "interceptors")]
    pub fn add_interceptor_fn<Req, Res, H>(self, handler: H) -> Self
    where
        Req: Send + 'static,
        Res: Send + 'static,
        H: FnMut(
                Req,
                Box<dyn FnOnce(Req) -> BoxFuture<'static, Res> + Send>,
            ) -> BoxFuture<'static, Res>
            + Send
            + 'static,
    {
        let req_ty = TypeId::of::<Req>();
        let res_ty = TypeId::of::<Res>();
        let key = InterceptorKey { req_ty, res_ty };

        let mediator = self.inner.clone();

        // FIXME: Find a way to prevent using async in the builder
        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let mut handlers_lock = mediator.interceptors.lock().await;
                let interceptors = handlers_lock.entry(key).or_insert(Vec::new());
                interceptors.push(InterceptorWrapper::from_handler_fn(handler));
                drop(handlers_lock);
            });
        });

        self
    }

    /// Adds a stream request interceptor.
    #[cfg(feature = "streams")]
    pub fn add_interceptor_stream<Req, T, S, H>(self, handler: H) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + Send + 'static,
        S: Stream<Item = T> + Send + 'static,
        T: Send + 'static,
        H: AsyncStreamInterceptor<Request = Req, Stream = S, Item = T> + Send + 'static,
    {
        let key = InterceptorKey::of::<Req, S>();
        let mediator = self.inner.clone();

        // FIXME: Find a way to prevent using async in the builder
        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let mut handlers_lock = mediator.interceptors.lock().await;
                let interceptors = handlers_lock.entry(key).or_insert(Vec::new());
                interceptors.push(InterceptorWrapper::from_stream(handler));
                drop(handlers_lock);
            });
        });

        self
    }

    /// Adds a stream request interceptor from a function.
    #[cfg(feature = "streams")]
    pub fn add_interceptor_stream_fn<Req, T, S, H>(self, handler: H) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + Send + 'static,
        S: Stream<Item = T> + Send + 'static,
        T: Send + 'static,
        H: FnMut(
                Req,
                Box<dyn FnOnce(Req) -> BoxFuture<'static, S> + Send>,
            ) -> BoxFuture<'static, S>
            + Send
            + 'static,
    {
        let key = InterceptorKey::of::<Req, S>();
        let mediator = self.inner.clone();

        // FIXME: Find a way to prevent using async in the builder
        // We block the thread to keep the api signature consistent and don't require await
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let mut handlers_lock = mediator.interceptors.lock().await;
                let interceptors = handlers_lock.entry(key).or_insert(Vec::new());
                interceptors.push(InterceptorWrapper::from_stream_fn(handler));
                drop(handlers_lock);
            });
        });

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

/// Assert the `DefaultMediator` is `Send + Sync`.
/// ```rust
/// use mediator::DefaultAsyncMediator;
///
/// fn assert_send_sync<T: Send + Sync>(t: T) {
///     drop(t);
/// }
///
/// let mediator = DefaultAsyncMediator::builder().build();
/// assert_send_sync(mediator);
/// ```
#[cfg(test)]
fn _dummy() {
    fn assert_send_sync<T: Send + Sync>(_: T) {}
    assert_send_sync(DefaultAsyncMediator::builder().build());
}

#[cfg(test)]
mod test {
    use crate::futures::BoxStream;
    use crate::{
        box_stream, AsyncMediator, AsyncRequestHandler, DefaultAsyncMediator, Event, Request,
        StreamRequest,
    };
    use std::marker::PhantomData;
    use std::sync::atomic::AtomicI64;
    use std::sync::{Arc, Mutex};
    use tokio_stream::StreamExt;

    #[tokio::test(flavor = "multi_thread")]
    async fn send_test() {
        struct WaitAndGetRequest<T>(T);
        impl<T: Send> Request<T> for WaitAndGetRequest<T> {}

        struct WaitAndSendRequestHandler<T>(PhantomData<T>);

        #[async_trait::async_trait]
        impl<T: Send> AsyncRequestHandler<WaitAndGetRequest<T>, T> for WaitAndSendRequestHandler<T> {
            async fn handle(&mut self, req: WaitAndGetRequest<T>) -> T {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                req.0
            }
        }

        let mut mediator = DefaultAsyncMediator::builder()
            .add_handler(WaitAndSendRequestHandler::<i32>(PhantomData))
            .add_handler(WaitAndSendRequestHandler::<String>(PhantomData))
            .build();

        let res1 = mediator.send(WaitAndGetRequest(1)).await.unwrap();
        assert_eq!(res1, 1);

        let res2 = mediator
            .send(WaitAndGetRequest("hello".to_owned()))
            .await
            .unwrap();
        assert_eq!(res2, "hello".to_owned());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn publish_test() {
        #[derive(Clone)]
        struct IncEvent(u32);
        impl Event for IncEvent {}

        #[derive(Clone)]
        struct DecEvent(u32);
        impl Event for DecEvent {}

        static VALUE: AtomicI64 = AtomicI64::new(0);

        let mut mediator = DefaultAsyncMediator::builder()
            .subscribe_fn(|event: IncEvent| async move {
                VALUE.fetch_add(event.0 as i64, std::sync::atomic::Ordering::SeqCst);
            })
            .subscribe_fn(|event: DecEvent| async move {
                VALUE.fetch_sub(event.0 as i64, std::sync::atomic::Ordering::SeqCst);
            })
            .build();

        mediator.publish(IncEvent(1)).await.unwrap();
        mediator.publish(IncEvent(2)).await.unwrap();
        mediator.publish(IncEvent(3)).await.unwrap();
        mediator.publish(DecEvent(2)).await.unwrap();

        assert_eq!(VALUE.load(std::sync::atomic::Ordering::SeqCst), 4);
    }

    #[tokio::test(flavor = "multi_thread")]
    #[cfg(feature = "streams")]
    async fn stream_test() {
        struct CounterRequest(u64);
        impl StreamRequest for CounterRequest {
            type Stream = BoxStream<'static, u64>;
            type Item = u64;
        }

        let mut mediator = DefaultAsyncMediator::builder()
            .add_stream_handler_fn(|req: CounterRequest| {
                box_stream! { yx move =>
                    for i in 0..req.0 {
                        yx.yield_one(i);
                    }
                }
            })
            .build();

        let mut stream = mediator.stream(CounterRequest(4)).await.unwrap();

        assert_eq!(stream.next().await.unwrap(), 0);
        assert_eq!(stream.next().await.unwrap(), 1);
        assert_eq!(stream.next().await.unwrap(), 2);
        assert_eq!(stream.next().await.unwrap(), 3);
        assert!(stream.next().await.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn add_handler_fn_with_test() {
        struct CountRequest(u32);
        impl Request<()> for CountRequest {}

        let counter = Arc::new(Mutex::new(0));

        let mut mediator = DefaultAsyncMediator::builder()
            .add_handler_fn_with(counter.clone(), |req: CountRequest, c| async move {
                *c.lock().unwrap() += req.0;
            })
            .build();

        mediator.send(CountRequest(1)).await.unwrap();
        mediator.send(CountRequest(2)).await.unwrap();
        mediator.send(CountRequest(3)).await.unwrap();

        assert_eq!(*counter.lock().unwrap(), 6);
    }
}
