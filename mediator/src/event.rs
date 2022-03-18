
/// Represents an application event.
pub trait Event : Clone {}

/// A handler for application events.
pub trait EventHandler<E: Event> {
    /// Handles an event.
    fn handle(&mut self, event: E);
}