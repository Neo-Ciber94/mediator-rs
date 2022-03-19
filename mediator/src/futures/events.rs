use crate::Event;

/// A handler for application events.
#[async_trait::async_trait]
pub trait EventHandler<E: Event> {
    /// Handles an event.
    async fn handle(&mut self, event: E);
}
