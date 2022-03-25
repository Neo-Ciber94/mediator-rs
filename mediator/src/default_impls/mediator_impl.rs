use crate::error::{Error, ErrorKind};
use crate::{Event, EventHandler, Mediator, Request, RequestHandler};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[cfg(feature = "streams")]
use tokio_stream::Stream;

#[cfg(feature = "streams")]
use crate::{StreamRequest, StreamRequestHandler};

type SharedHandler<H> = Arc<Mutex<HashMap<TypeId, H>>>;

// A wrapper around the request handler to handle the request and return the result.
// To provide type safety without unsafe code we box all: the function, the params and the result.
#[derive(Clone)]
struct RequestHandlerWrapper {
    #[allow(clippy::type_complexity)]
    handler: Arc<Mutex<dyn FnMut(Box<dyn Any>) -> Box<dyn Any>>>,
    is_deferred: bool,
}

impl RequestHandlerWrapper {
    pub fn new<Req, Res, H>(mut handler: H) -> Self
    where
        Res: 'static,
        Req: Request<Res> + 'static,
        H: RequestHandler<Req, Res> + 'static,
    {
        let f = move |req: Box<dyn Any>| -> Box<dyn Any> {
            let req = *req.downcast::<Req>().unwrap();
            Box::new(handler.handle(req))
        };

        RequestHandlerWrapper {
            handler: Arc::new(Mutex::new(f)),
            is_deferred: false,
        }
    }

    pub fn from_fn<Req, Res, F>(mut handler: F) -> Self
    where
        Res: 'static,
        Req: Request<Res> + 'static,
        F: FnMut(Req) -> Res + 'static,
    {
        let f = move |req: Box<dyn Any>| -> Box<dyn Any> {
            let req = *req.downcast::<Req>().unwrap();
            Box::new(handler(req))
        };

        RequestHandlerWrapper {
            handler: Arc::new(Mutex::new(f)),
            is_deferred: false,
        }
    }

    pub fn from_deferred<Res, Req, F>(mut handler: F) -> Self
    where
        Res: 'static,
        Req: Request<Res> + 'static,
        F: FnMut(Req, DefaultMediator) -> Res + 'static,
    {
        let f = move |args: Box<dyn Any>| -> Box<dyn Any> {
            let (req, mediator) = *args.downcast::<(Req, DefaultMediator)>().unwrap();
            Box::new(handler(req, mediator))
        };

        RequestHandlerWrapper {
            handler: Arc::new(Mutex::new(f)),
            is_deferred: true,
        }
    }

    pub fn handle<Req, Res>(&mut self, req: Req, mediator: Option<DefaultMediator>) -> Option<Res>
    where
        Res: 'static,
        Req: Request<Res> + 'static,
    {
        let mut handler = self.handler.lock().unwrap();
        let req: Box<dyn Any> = match mediator {
            Some(mediator) => Box::new((req, mediator)),
            None => Box::new(req),
        };

        let res = (handler)(req);
        res.downcast::<Res>().map(|res| *res).ok()
    }
}

// A wrapper around the event handler to handle the events.
// To provide type safety without unsafe code we box all: the function, the params and the result.
#[derive(Clone)]
struct EventHandlerWrapper {
    #[allow(clippy::type_complexity)]
    handler: Arc<Mutex<dyn FnMut(Box<dyn Any>)>>,
    is_deferred: bool,
}

impl EventHandlerWrapper {
    pub fn new<E, H>(mut handler: H) -> Self
    where
        E: Event + 'static,
        H: EventHandler<E> + 'static,
    {
        let f = move |event: Box<dyn Any>| {
            let event = *event.downcast::<E>().unwrap();
            handler.handle(event);
        };

        EventHandlerWrapper {
            handler: Arc::new(Mutex::new(f)),
            is_deferred: false,
        }
    }

    pub fn from_fn<E, F>(mut handler: F) -> Self
    where
        E: Event + 'static,
        F: FnMut(E) + 'static,
    {
        let f = move |event: Box<dyn Any>| {
            let event = *event.downcast::<E>().unwrap();
            handler(event);
        };

        EventHandlerWrapper {
            handler: Arc::new(Mutex::new(f)),
            is_deferred: false,
        }
    }

    pub fn from_deferred<E, F>(mut handler: F) -> Self
    where
        E: Event + 'static,
        F: FnMut(E, DefaultMediator) + 'static,
    {
        let f = move |args: Box<dyn Any>| {
            let (event, mediator) = *args.downcast::<(E, DefaultMediator)>().unwrap();
            handler(event, mediator);
        };

        EventHandlerWrapper {
            handler: Arc::new(Mutex::new(f)),
            is_deferred: true,
        }
    }

    pub fn handle<E>(&mut self, event: E, mediator: Option<DefaultMediator>)
    where
        E: Event + 'static,
    {
        let mut handler = self.handler.lock().unwrap();
        let event: Box<dyn Any> = match mediator {
            Some(mediator) => Box::new((event, mediator)),
            None => Box::new(event),
        };

        (handler)(event);
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

    pub fn from_fn_with<State, Req, S, T, F>(mut handler: F, state: State) -> Self
        where
            State: Send + Clone + 'static,
            Req: StreamRequest<Stream = S, Item = T> + 'static,
            S: Stream<Item = T> + 'static,
            F: FnMut(Req, State) -> S + 'static,
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
        F: FnMut(Req, DefaultMediator) -> S + 'static,
        T: 'static,
    {
        let f = move |req: Box<dyn Any>| -> Box<dyn Any> {
            let (req, mediator) = *req.downcast::<(Req, DefaultMediator)>().unwrap();
            Box::new(handler(req, mediator))
        };

        StreamRequestHandlerWrapper {
            handler: Arc::new(Mutex::new(f)),
            is_deferred: true,
        }
    }

    pub fn from_deferred_with<State, Req, S, T, F>(mut handler: F, state: State) -> Self
        where
            State: Send + Clone + 'static,
            Req: StreamRequest<Stream = S, Item = T> + 'static,
            S: Stream<Item = T> + 'static,
            F: FnMut(Req, DefaultMediator, State) -> S + 'static,
            T: 'static,
    {
        let f = move |req: Box<dyn Any>| -> Box<dyn Any> {
            let (req, mediator) = *req.downcast::<(Req, DefaultMediator)>().unwrap();
            Box::new(handler(req, mediator, state.clone()))
        };

        StreamRequestHandlerWrapper {
            handler: Arc::new(Mutex::new(f)),
            is_deferred: true,
        }
    }

    pub fn handle<Req, S, T>(&mut self, req: Req, mediator: Option<DefaultMediator>) -> Option<S>
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

/// A default implementation for the [Mediator] trait.
///
/// # Examples
///
/// ## Request handler
/// ```
/// use std::sync::atomic::AtomicU64;
/// use mediator::{DefaultMediator, Mediator, Request, RequestHandler};
///
/// struct GetNextId;
/// impl Request<u64> for GetNextId { }
///
/// struct GetNextIdHandler;
/// impl RequestHandler<GetNextId, u64> for GetNextIdHandler {
///   fn handle(&mut self, _: GetNextId) -> u64 {
///     static NEXT_ID : AtomicU64 = AtomicU64::new(1);
///     NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
///   }
/// }
///
/// let mut mediator = DefaultMediator::builder()
///     .add_handler(GetNextIdHandler)
///     .build();
///
/// assert_eq!(Ok(1), mediator.send(GetNextId));
/// assert_eq!(Ok(2), mediator.send(GetNextId));
/// assert_eq!(Ok(3), mediator.send(GetNextId));
/// ```
///
/// ## Event handler
/// ```
/// use mediator::{Event, DefaultMediator, Mediator};
///
/// #[derive(Clone)]
/// struct Product { name: String };
///
/// #[derive(Clone)]
/// struct ProductAddedEvent(Product);
/// impl Event for ProductAddedEvent { }
///
/// struct ProductService(Vec<Product>, DefaultMediator);
/// impl ProductService {
///     pub fn add<S: Into<String>>(&mut self, product: S) {
///         let product = Product { name: product.into() };
///         self.0.push(product.clone());
///         self.1.publish(ProductAddedEvent(product));
///     }
/// }
///
/// let mut mediator = DefaultMediator::builder()
///     .subscribe_fn(move |event: ProductAddedEvent| {
///         println!("Product added: {}", event.0.name);
///     })
///    .build();
///
/// let mut service = ProductService(vec![], mediator.clone());
///
/// service.add("Microwave");   // Product added: Microwave
/// service.add("Toaster");     // Product added: Toaster
/// ```
#[derive(Clone)]
pub struct DefaultMediator {
    request_handlers: SharedHandler<RequestHandlerWrapper>,
    event_handlers: SharedHandler<Vec<EventHandlerWrapper>>,

    #[cfg(feature = "streams")]
    stream_handlers: SharedHandler<StreamRequestHandlerWrapper>,
}

// SAFETY: the `request_handlers` and `event_handlers` are wrapped in Arc and Mutex.
unsafe impl Send for DefaultMediator {}

// SAFETY: the `request_handlers` and `event_handlers` are wrapped in Arc and Mutex.
unsafe impl Sync for DefaultMediator {}

impl DefaultMediator {
    /// Gets a [DefaultMediator] builder.
    pub fn builder() -> Builder {
        Builder::new()
    }
}

impl Mediator for DefaultMediator {
    fn send<Req, Res>(&mut self, req: Req) -> crate::Result<Res>
    where
        Res: 'static,
        Req: Request<Res> + 'static,
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

            if let Some(res) = handler.handle(req, mediator) {
                return Ok(res);
            }
        }

        Err(Error::from(ErrorKind::NotFound))
    }

    fn publish<E>(&mut self, event: E) -> crate::Result<()>
    where
        E: Event + 'static,
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

                handler.handle(event.clone(), mediator);
            }

            Ok(())
        } else {
            Err(Error::from(ErrorKind::NotFound))
        }
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

/// A builder for the [DefaultMediator].
pub struct Builder {
    inner: DefaultMediator,
}

impl Builder {
    /// Constructs a new `DefaultMediatorBuilder`.
    pub fn new() -> Self {
        Builder {
            inner: DefaultMediator {
                request_handlers: SharedHandler::default(),
                event_handlers: SharedHandler::default(),

                #[cfg(feature = "streams")]
                stream_handlers: SharedHandler::default(),
            },
        }
    }
    
    /// Registers a request handler.
    pub fn add_handler<Req, Res, H>(self, handler: H) -> Self
    where
        Res: 'static,
        Req: Request<Res> + 'static,
        H: RequestHandler<Req, Res> + 'static,
    {
        let mut handlers_lock = self.inner.request_handlers.lock().unwrap();
        handlers_lock.insert(TypeId::of::<Req>(), RequestHandlerWrapper::new(handler));
        drop(handlers_lock);
        self
    }

    /// Registers a request handler from a function.
    pub fn add_handler_fn<Req, Res, F>(self, handler: F) -> Self
    where
        Res: 'static,
        Req: Request<Res> + 'static,
        F: FnMut(Req) -> Res + 'static,
    {
        let mut handlers_lock = self.inner.request_handlers.lock().unwrap();
        handlers_lock.insert(TypeId::of::<Req>(), RequestHandlerWrapper::from_fn(handler));
        drop(handlers_lock);
        self
    }

    /// Register a request handler using a copy of the mediator.
    pub fn add_handler_deferred<Req, Res, H, F>(self, f: F) -> Self
    where
        Res: 'static,
        Req: Request<Res> + 'static,
        H: RequestHandler<Req, Res> + 'static,
        F: Fn(DefaultMediator) -> H,
    {
        let handler = f(self.inner.clone());
        self.add_handler(handler)
    }

    /// Registers a request handler from a function using a copy of the mediator.
    pub fn add_handler_fn_deferred<Req, Res, F>(self, f: F) -> Self
    where
        Res: 'static,
        Req: Request<Res> + 'static,
        F: FnMut(Req, DefaultMediator) -> Res + 'static,
    {
        let mut handlers_lock = self.inner.request_handlers.lock().unwrap();
        handlers_lock.insert(TypeId::of::<Req>(), RequestHandlerWrapper::from_deferred(f));
        drop(handlers_lock);
        self
    }

    /// Registers an event handler.
    pub fn subscribe<E, H>(self, handler: H) -> Self
    where
        E: Event + 'static,
        H: EventHandler<E> + 'static,
    {
        let mut handlers_lock = self.inner.event_handlers.lock().unwrap();
        let event_handlers = handlers_lock
            .entry(TypeId::of::<E>())
            .or_insert_with(Vec::new);
        event_handlers.push(EventHandlerWrapper::new(handler));
        drop(handlers_lock);
        self
    }

    /// Registers an event handler from a function.
    pub fn subscribe_fn<E, F>(self, handler: F) -> Self
    where
        E: Event + 'static,
        F: FnMut(E) + 'static,
    {
        let mut handlers_lock = self.inner.event_handlers.lock().unwrap();
        let event_handlers = handlers_lock
            .entry(TypeId::of::<E>())
            .or_insert_with(Vec::new);
        event_handlers.push(EventHandlerWrapper::from_fn(handler));
        drop(handlers_lock);
        self
    }

    /// Registers an event handler using a copy of the mediator.
    pub fn subscribe_deferred<E, H, F>(self, f: F) -> Self
    where
        E: Event + 'static,
        H: EventHandler<E> + 'static,
        F: Fn(DefaultMediator) -> H,
    {
        let handler = f(self.inner.clone());
        self.subscribe(handler)
    }

    /// Registers an event handler from a function using a copy of the mediator.
    pub fn subscribe_fn_deferred<E, H, F>(self, f: F) -> Self
    where
        E: Event + 'static,
        F: FnMut(E, DefaultMediator) + 'static,
    {
        let mut handlers_lock = self.inner.event_handlers.lock().unwrap();
        let event_handlers = handlers_lock
            .entry(TypeId::of::<E>())
            .or_insert_with(Vec::new);
        event_handlers.push(EventHandlerWrapper::from_deferred(f));
        drop(handlers_lock);
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

    #[cfg(feature = "streams")]
    pub fn add_stream_handler_fn_with<State, Req, S, T, F>(self, state: State, f: F) -> Self
        where
            State: Send + Clone + 'static,
            Req: StreamRequest<Stream = S, Item = T> + 'static,
            F: FnMut(Req, State) -> S + 'static,
            S: Stream<Item = T> + 'static,
            T: 'static,
    {
        let mut handlers_lock = self.inner.stream_handlers.lock().unwrap();
        handlers_lock.insert(TypeId::of::<Req>(), StreamRequestHandlerWrapper::from_fn_with(f, state));
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
        F: Fn(DefaultMediator) -> H,
    {
        let handler = f(self.inner.clone());
        self.add_stream_handler(handler)
    }

    /// Registers a stream handler from a function using a copy of the mediator.
    #[cfg(feature = "streams")]
    pub fn add_stream_handler_fn_deferred<Req, S, T, F>(self, f: F) -> Self
    where
        Req: StreamRequest<Stream = S, Item = T> + 'static,
        F: FnMut(Req, DefaultMediator) -> S + 'static,
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
    pub fn add_stream_handler_fn_deferred_with<State, Req, S, T, F>(self, state: State, f: F) -> Self
        where
            State: Send + Clone + 'static,
            Req: StreamRequest<Stream = S, Item = T> + 'static,
            F: FnMut(Req, DefaultMediator, State) -> S + 'static,
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

    /// Builds the `DefaultMediator`.
    pub fn build(self) -> DefaultMediator {
        self.inner
    }
}

impl Default for Builder {
    fn default() -> Self {
        Builder::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{DefaultMediator, Event, EventHandler, Mediator, Request, RequestHandler};
    use std::ops::Range;
    use std::sync::atomic::AtomicUsize;

    #[cfg(feature = "streams")]
    use crate::{StreamRequest, StreamRequestHandler};

    #[test]
    fn send_request_test() {
        struct TwoTimesRequest(i64);
        impl Request<i64> for TwoTimesRequest {}

        struct TwoTimesRequestHandler;
        impl RequestHandler<TwoTimesRequest, i64> for TwoTimesRequestHandler {
            fn handle(&mut self, request: TwoTimesRequest) -> i64 {
                request.0 * 2
            }
        }

        let mut mediator = DefaultMediator::builder()
            .add_handler(TwoTimesRequestHandler)
            .build();

        assert_eq!(4, mediator.send(TwoTimesRequest(2)).unwrap());
        assert_eq!(-6, mediator.send(TwoTimesRequest(-3)).unwrap());
    }

    #[test]
    fn publish_event_test() {
        #[derive(Clone)]
        struct IncrementEvent;
        impl Event for IncrementEvent {}

        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        struct TestEventHandler;
        impl EventHandler<IncrementEvent> for TestEventHandler {
            fn handle(&mut self, _: IncrementEvent) {
                COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }

        let mut mediator = DefaultMediator::builder()
            .subscribe(TestEventHandler)
            .build();

        mediator.publish(IncrementEvent).unwrap();
        mediator.publish(IncrementEvent).unwrap();
        assert_eq!(2, COUNTER.load(std::sync::atomic::Ordering::SeqCst));

        mediator.publish(IncrementEvent).unwrap();
        assert_eq!(3, COUNTER.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test(flavor = "multi_thread")]
    #[cfg(feature = "streams")]
    async fn stream_test() {
        use tokio_stream::StreamExt;

        struct CounterRequest(u32);
        impl StreamRequest for CounterRequest {
            type Stream = tokio_stream::Iter<Range<u32>>;
            type Item = u32;
        }

        struct CounterRequestHandler;
        impl StreamRequestHandler for CounterRequestHandler {
            type Request = CounterRequest;
            type Stream = tokio_stream::Iter<Range<u32>>;
            type Item = u32;

            fn handle_stream(&mut self, req: CounterRequest) -> Self::Stream {
                tokio_stream::iter(0..req.0)
            }
        }

        let mut mediator = DefaultMediator::builder()
            .add_stream_handler(CounterRequestHandler)
            .build();

        let mut counter_stream = mediator.stream(CounterRequest(5)).unwrap();
        assert_eq!(0, counter_stream.next().await.unwrap());
        assert_eq!(1, counter_stream.next().await.unwrap());
        assert_eq!(2, counter_stream.next().await.unwrap());
        assert_eq!(3, counter_stream.next().await.unwrap());
        assert_eq!(4, counter_stream.next().await.unwrap());
        assert!(counter_stream.next().await.is_none());
    }
}
