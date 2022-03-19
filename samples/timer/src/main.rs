use mediator::streams::Stream;
use mediator::{streams, streams::StreamExt, DefaultMediator, Event, StreamRequest, StreamRequestHandler, Mediator};
use std::pin::Pin;
use std::time::Duration;

type BoxStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

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
        let stream = streams::iter(0..req.0)
            .throttle(Duration::from_secs(1));

        Box::pin(stream)
    }
}

#[derive(Clone)]
struct SecondPassedEvent(u64);
impl Event for SecondPassedEvent {}

#[tokio::main]
async fn main() {
    let mut mediator = DefaultMediator::builder()
        .add_stream_handler_deferred(TimerRequestHandler)
        .subscribe_fn(|event: SecondPassedEvent| {
            println!("{} second passed", event.0 + 1);
        })
        .build();

    let mut stream = mediator.stream(TimerRequest(5)).unwrap();
    while let Some(item) = stream.next().await {
        mediator.publish(SecondPassedEvent(item)).unwrap();
    }
}