use mediator::{Request, RequestHandler};
use crate::BoxedProductService;
use crate::models::Product;

pub struct GetAllProductsQuery;
impl Request<Vec<Product>> for GetAllProductsQuery {}

pub struct GetAllProductsHandler(pub BoxedProductService);
impl RequestHandler<GetAllProductsQuery, Vec<Product>> for GetAllProductsHandler {
    fn handle(&mut self, _: GetAllProductsQuery) -> Vec<Product> {
        self.0.lock().unwrap().get_all()
    }
}
