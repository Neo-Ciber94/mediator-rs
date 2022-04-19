use crate::Request;

// pub enum Ordering {
//     Before,
//     After,
// }

pub trait Interceptor<Req, Res>
where
    Req: Request<Res>,
{
    fn handle(&mut self, req: Req, next: Box<dyn FnOnce(Req) -> Res>) -> Res;
}

#[async_trait::async_trait]
pub trait AsyncInterceptor<Req, Res>
where
    Req: Request<Res> + Send,
{
    async fn handle(&mut self, req: Req, next: Box<dyn FnOnce(Req) -> Res>) -> Res;
}
