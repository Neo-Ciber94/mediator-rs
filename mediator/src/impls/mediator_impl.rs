use crate::{Error, Event, EventHandler, Mediator, Request, RequestHandler};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type SharedHandler<H> = Arc<Mutex<HashMap<TypeId, H>>>;

// A wrapper around the request handler to handle the request and return the result.
// To provide type safety without unsafe code we box all: the function, the params and the result.
struct RequestHandlerWrapper {
    handler: Box<dyn FnMut(Box<dyn Any>) -> Box<dyn Any>>,
}

impl RequestHandlerWrapper {
    pub fn new<Req, Res, H>(mut handler: H) -> Self
    where
        Res: 'static,
        Req: Request<Res> + 'static,
        H: RequestHandler<Req, Res> + 'static,
    {
        RequestHandlerWrapper {
            handler: Box::new(move |req| {
                let req = *req.downcast::<Req>().unwrap();
                Box::new(handler.handle(req))
            }),
        }
    }

    pub fn from_fn<Req, Res, F>(mut handler: F) -> Self
    where
        Res: 'static,
        Req: Request<Res> + 'static,
        F: FnMut(Req) -> Res + 'static,
    {
        RequestHandlerWrapper {
            handler: Box::new(move |req| {
                let req = *req.downcast::<Req>().unwrap();
                Box::new(handler(req))
            }),
        }
    }

    pub fn handle<Req, Res>(&mut self, req: Req) -> Option<Res>
    where
        Res: 'static,
        Req: Request<Res> + 'static,
    {
        let req = Box::new(req);
        let res = (self.handler)(req);
        res.downcast::<Res>().map(|res| *res).ok()
    }
}

// A wrapper around the event handler to handle the events.
// To provide type safety without unsafe code we box all: the function, the params and the result.
struct EventHandlerWrapper {
    handler: Box<dyn FnMut(Box<dyn Any>)>,
}

impl EventHandlerWrapper {
    pub fn new<E, H>(mut handler: H) -> Self
    where
        E: Event + 'static,
        H: EventHandler<E> + 'static,
    {
        EventHandlerWrapper {
            handler: Box::new(move |event| {
                let event = *event.downcast::<E>().unwrap();
                handler.handle(event);
            }),
        }
    }

    pub fn from_fn<E, F>(mut handler: F) -> Self
    where
        E: Event + 'static,
        F: FnMut(E) + 'static,
    {
        EventHandlerWrapper {
            handler: Box::new(move |event| {
                let event = *event.downcast::<E>().unwrap();
                handler(event);
            }),
        }
    }

    pub fn handle<E>(&mut self, event: E)
    where
        E: Event + 'static,
    {
        let event = Box::new(event);
        (self.handler)(event);
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
/// let mut mediator = DefaultMediator::new();
/// mediator.add_handler(GetNextIdHandler);
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
/// let mut mediator = DefaultMediator::new();
/// let mut service = ProductService(vec![], mediator.clone());
///
/// mediator.subscribe_fn(move |event: ProductAddedEvent| {
///     println!("Product added: {}", event.0.name);
/// });
///
/// service.add("Microwave");   // Product added: Microwave
/// service.add("Toaster");     // Product added: Toaster
/// ```
///
#[derive(Clone)]
pub struct DefaultMediator {
    request_handlers: SharedHandler<RequestHandlerWrapper>,
    event_handlers: SharedHandler<Vec<EventHandlerWrapper>>,
}

impl DefaultMediator {
    /// Constructs a new `DefaultMediator`.
    pub fn new() -> Self {
        DefaultMediator {
            request_handlers: Arc::new(Mutex::new(HashMap::new())),
            event_handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Registers a request handler.
    pub fn add_handler<Req, Res, H>(&mut self, handler: H)
    where
        Res: 'static,
        Req: Request<Res> + 'static,
        H: RequestHandler<Req, Res> + 'static,
    {
        let mut handlers = self.request_handlers.lock().unwrap();
        handlers.insert(TypeId::of::<Req>(), RequestHandlerWrapper::new(handler));
    }

    /// Registers a request handler from a function.
    pub fn add_handler_fn<Req, Res, F>(&mut self, handler: F)
    where
        Res: 'static,
        Req: Request<Res> + 'static,
        F: FnMut(Req) -> Res + 'static,
    {
        let mut handlers = self.request_handlers.lock().unwrap();
        handlers.insert(TypeId::of::<Req>(), RequestHandlerWrapper::from_fn(handler));
    }

    /// Registers an event handler.
    pub fn subscribe<E, H>(&mut self, handler: H)
    where
        E: Event + 'static,
        H: EventHandler<E> + 'static,
    {
        let mut handlers = self.event_handlers.lock().unwrap();
        let event_handlers = handlers.entry(TypeId::of::<E>()).or_insert_with(Vec::new);

        event_handlers.push(EventHandlerWrapper::new(handler));
    }

    /// Registers an event handler from a function.
    pub fn subscribe_fn<E, F>(&mut self, handler: F)
    where
        E: Event + 'static,
        F: FnMut(E) + 'static,
    {
        let mut handlers = self.event_handlers.lock().unwrap();
        let event_handlers = handlers.entry(TypeId::of::<E>()).or_insert_with(Vec::new);

        event_handlers.push(EventHandlerWrapper::from_fn(handler));
    }
}

impl Mediator for DefaultMediator {
    fn send<Req, Res>(&mut self, req: Req) -> crate::Result<Res>
    where
        Res: 'static,
        Req: Request<Res> + 'static,
    {
        let type_id = TypeId::of::<Req>();
        let mut handlers = self.request_handlers.lock().unwrap();

        if let Some(handler) = handlers.get_mut(&type_id) {
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
        let mut handlers = self.event_handlers.lock().unwrap();

        if let Some(handlers) = handlers.get_mut(&type_id) {
            for handler in handlers {
                handler.handle(event.clone());
            }
        }

        Ok(())
    }
}
