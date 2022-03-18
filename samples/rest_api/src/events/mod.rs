use crate::models::Product;
use mediator::Event;

#[derive(Debug, Clone)]
pub struct ProductAddedEvent(pub Product);
impl Event for ProductAddedEvent {}
