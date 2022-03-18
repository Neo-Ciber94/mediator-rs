use crate::models::Product;
use crate::BoxedProductService;
use mediator::{Request, RequestHandler};

pub struct GetAllProductsQuery;
impl Request<Vec<Product>> for GetAllProductsQuery {}

pub struct GetAllProductsHandler(pub BoxedProductService);
impl RequestHandler<GetAllProductsQuery, Vec<Product>> for GetAllProductsHandler {
    fn handle(&mut self, _: GetAllProductsQuery) -> Vec<Product> {
        self.0.lock().unwrap().get_all()
    }
}
