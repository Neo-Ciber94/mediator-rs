use mediator::Event;
use crate::models::Product;

#[derive(Debug, Clone)]
pub struct ProductAddedEvent(pub Product);
impl Event for ProductAddedEvent {}