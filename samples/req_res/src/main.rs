use mediator::{DefaultMediator, Mediator, Request, RequestHandler};
use std::sync::atomic::AtomicU64;

struct GetNextIdRequest;
impl Request<u64> for GetNextIdRequest {}

struct GetNextTwoIdsRequest;
impl Request<(u64, u64)> for GetNextTwoIdsRequest {}

struct GetNextIdRequestHandler;
impl RequestHandler<GetNextIdRequest, u64> for GetNextIdRequestHandler {
    fn handle(&mut self, _: GetNextIdRequest) -> u64 {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

struct GetNextTwoIdsRequestHandler<M: Mediator>(M);
impl<M: Mediator> RequestHandler<GetNextTwoIdsRequest, (u64, u64)>
    for GetNextTwoIdsRequestHandler<M>
{
    fn handle(&mut self, _: GetNextTwoIdsRequest) -> (u64, u64) {
        let n1 = self.0.send(GetNextIdRequest).unwrap();
        let n2 = self.0.send(GetNextIdRequest).unwrap();
        (n1, n2)
    }
}

fn main() {
    let mut mediator = DefaultMediator::builder()
        .add_handler(GetNextIdRequestHandler)
        .add_handler_deferred(GetNextTwoIdsRequestHandler)
        .build();

    println!("{:?}", mediator.send(GetNextIdRequest).unwrap());
    println!("{:?}", mediator.send(GetNextIdRequest).unwrap());
    println!("{:?}", mediator.send(GetNextTwoIdsRequest).unwrap());
}
