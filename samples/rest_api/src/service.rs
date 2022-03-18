use std::collections::HashMap;
use uuid::Uuid;
use crate::models::Product;

#[derive(Debug, Clone)]
pub struct ProductService {
    products: HashMap<Uuid, Product>
}

impl ProductService {
    pub fn new() -> ProductService {
        ProductService {
            products: HashMap::new()
        }
    }

    pub fn add(&mut self, product: Product) -> Product {
        self.products.insert(product.id(), product.clone());
        product
    }

    pub fn update(&mut self, product: Product) -> Option<Product> {
        if let Some(p) = self.products.get_mut(&product.id()) {
            *p = product.clone();
            Some(product)
        } else {
            None
        }
    }

    pub fn delete(&mut self, id: Uuid) -> Option<Product> {
        self.products.remove(&id)
    }

    pub fn get(&self, id: Uuid) -> Option<Product> {
        self.products.get(&id).cloned()
    }

    pub fn get_all(&self) -> Vec<Product> {
        self.products.values().cloned().collect()
    }
}