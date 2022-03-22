use mediator::futures::Stream;
use mediator::{futures, futures::StreamExt, DefaultMediator, Event, Mediator, StreamRequest, StreamRequestHandler, box_stream};
use std::pin::Pin;
use std::time::Duration;

type BoxStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

#[derive(Clone)]
struct SecondPassedEvent(u64);
impl Event for SecondPassedEvent {}

struct TimerRequest(u64);
impl StreamRequest for TimerRequest {
    type Stream = BoxStream<u64>;
    type Item = u64;
}

struct TimerRequestHandler(DefaultMediator);
impl StreamRequestHandler for TimerRequestHandler {
    type Request = TimerRequest;
    type Stream = BoxStream<u64>;
    type Item = u64;

    fn handle_stream(&mut self, req: Self::Request) -> Self::Stream {
        box_stream! { yx move =>
            for i in 0..req.0 {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                let s = i + 1;
                yx.yield_one(s);
                self.0.publish(SecondPassedEvent(s)).expect("publish failed");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let mut mediator = DefaultMediator::builder()
        .add_stream_handler_deferred(TimerRequestHandler)
        .subscribe_fn(|event: SecondPassedEvent| {
            println!("{} second passed", event.0);
        })
        .build();

    let mut stream = mediator.stream(TimerRequest(5)).unwrap();
    while let Some(_) = stream.next().await {}
}
