use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Product {
    id: Uuid,
    name: String,
    price: f64,
}

impl Product {
    pub fn new(name: &str, price: f64) -> Product {
        let name: String = name.into();

        Product {
            id: Uuid::new_v4(),
            name,
            price
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn price(&self) -> f64 {
        self.price
    }
}