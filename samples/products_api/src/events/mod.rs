use crate::Product;
use mediator::Event;

#[derive(Debug, Clone)]
pub struct ProductAddedEvent(pub Product);
impl Event for ProductAddedEvent {}

#[derive(Debug, Clone)]
pub struct ProductUpdatedEvent(pub Product);
impl Event for ProductUpdatedEvent {}

#[derive(Debug, Clone)]
pub struct ProductDeletedEvent(pub Product);
impl Event for ProductDeletedEvent {}
