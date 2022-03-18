use crate::{Error, Event, EventHandler, Mediator, Request, RequestHandler};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type SharedHandler<H> = Arc<Mutex<HashMap<TypeId, H>>>;

// A wrapper around the request handler to handle the request and return the result.
// To provide type safety without unsafe code we box all: the function, the params and the result.
#[derive(Clone)]
struct RequestHandlerWrapper {
    #[allow(clippy::type_complexity)]
    handler: Arc<Mutex<dyn FnMut(Box<dyn Any>) -> Box<dyn Any>>>,
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
        }
    }

    pub fn from_fn<Req, Res, F>(mut handler: F) -> Self
    where
        Res: 'static,
        Req: Request<Res> + 'static,
        F: FnMut(Req) -> Res + 'static,
    {
        let f = move |req: Box<dyn Any>| -> Box<dyn Any> {
            let req: Req = *req.downcast::<Req>().unwrap();
            Box::new(handler(req))
        };

        RequestHandlerWrapper {
            handler: Arc::new(Mutex::new(f)),
        }
    }

    pub fn handle<Req, Res>(&mut self, req: Req) -> Option<Res>
    where
        Res: 'static,
        Req: Request<Res> + 'static,
    {
        let req = Box::new(req);
        let mut handler = self.handler.lock().unwrap();
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
        }
    }

    pub fn handle<E>(&mut self, event: E)
    where
        E: Event + 'static,
    {
        let event = Box::new(event);
        let mut handler = self.handler.lock().unwrap();
        (handler)(event);
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
}

// SAFETY: the `request_handlers` and `event_handlers` are wrapped in Arc and Mutex.
unsafe impl Send for DefaultMediator {}

// SAFETY: the `request_handlers` and `event_handlers` are wrapped in Arc and Mutex.
unsafe impl Sync for DefaultMediator {}

impl DefaultMediator {
    // Gets a [DefaultMediator] builder.
    pub fn builder() -> DefaultMediatorBuilder {
        DefaultMediatorBuilder::new()
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

            if let Some(res) = handler.handle(req) {
                return Ok(res);
            }
        }

        Err(Error::NotFound)
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

        if let Some(handlers) = handlers_lock.get_mut(&type_id).cloned() {
            // Drop the lock to avoid deadlocks
            drop(handlers_lock);

            for mut handler in handlers {
                handler.handle(event.clone());
            }
        }

        Ok(())
    }
}

/// A builder for the [DefaultMediator].
pub struct DefaultMediatorBuilder {
    inner: DefaultMediator,
}

impl DefaultMediatorBuilder {
    /// Constructs a new `DefaultMediatorBuilder`.
    pub fn new() -> Self {
        DefaultMediatorBuilder {
            inner: DefaultMediator {
                request_handlers: SharedHandler::default(),
                event_handlers: SharedHandler::default(),
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
    pub fn add_handler_fn_deferred<Req, Res, H, F>(self, f: F) -> Self
    where
        Res: 'static,
        Req: Request<Res> + 'static,
        H: FnMut(Req) -> Res + 'static,
        F: Fn(DefaultMediator) -> H + 'static,
    {
        let handler = f(self.inner.clone());
        self.add_handler_fn(handler)
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
        H: FnMut(E) + 'static,
        F: Fn(DefaultMediator) -> H + 'static,
    {
        let handler = f(self.inner.clone());
        self.subscribe_fn(handler)
    }

    /// Builds a `DefaultMediator`.
    pub fn build(self) -> DefaultMediator {
        self.inner
    }
}

impl Default for DefaultMediatorBuilder {
    fn default() -> Self {
        DefaultMediatorBuilder::new()
    }
}