use crate::models::product::Product;
use crate::SharedRedisService;
use mediator::{Request, RequestHandler};

pub struct GetAllProductsRequest;
impl Request<Vec<Product>> for GetAllProductsRequest {}

pub struct GetAllProductsRequestHandler(pub SharedRedisService<Product>);
impl RequestHandler<GetAllProductsRequest, Vec<Product>> for GetAllProductsRequestHandler {
    fn handle(&mut self, _: GetAllProductsRequest) -> Vec<Product> {
        self.0
            .lock()
            .expect("Failed to lock redis service")
            .get_all()
            .expect("Failed to get all products")
    }
}
