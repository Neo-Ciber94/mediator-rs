use crate::{KeyPressRequest, KeyPressResult};
use mediator::Interceptor;

pub struct UpperCaseInterceptor;
impl Interceptor<KeyPressRequest, KeyPressResult> for UpperCaseInterceptor {
    fn handle(
        &mut self,
        req: KeyPressRequest,
        next: Box<dyn FnOnce(KeyPressRequest) -> KeyPressResult>,
    ) -> KeyPressResult {
        let res = next(req);
        res.map(|s| s.to_uppercase())
    }
}
