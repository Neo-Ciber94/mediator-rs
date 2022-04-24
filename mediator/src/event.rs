/// Represents an application event.
pub trait Event: Clone {}

/// A handler for application events.
pub trait EventHandler<E: Event> {
    /// Handles an event.
    fn handle(&mut self, event: E);
}

/// An async handler for application events.
#[cfg(feature = "async")]
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait AsyncEventHandler<E: Event> {
    /// Handles an event.
    async fn handle(&mut self, event: E);
}

///////////////////// Implementations /////////////////////

impl<E, F> EventHandler<E> for F
where
    E: Event,
    F: FnMut(E),
{
    fn handle(&mut self, event: E) {
        self(event)
    }
}

#[cfg(feature = "async")]
#[cfg_attr(feature = "async", async_trait::async_trait)]
impl<E, F> AsyncEventHandler<E> for F
where
    E: Event + Send + 'static,
    F: FnMut(E) -> crate::futures::BoxFuture<'static, ()> + Send,
{
    async fn handle(&mut self, event: E) {
        self(event).await
    }
}
