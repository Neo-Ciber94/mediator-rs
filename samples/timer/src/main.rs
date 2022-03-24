use mediator::futures::{BoxStream, StreamExt};
use mediator::{box_stream, DefaultMediator, Event, Mediator, StreamRequest, StreamRequestHandler};
use std::time::Duration;

#[derive(Clone)]
struct SecondPassedEvent(u64);
impl Event for SecondPassedEvent {}

struct TimerRequest(u64);
impl StreamRequest for TimerRequest {
    type Stream = BoxStream<'static, u64>;
    type Item = u64;
}

struct TimerRequestHandler(DefaultMediator);
impl StreamRequestHandler for TimerRequestHandler {
    type Request = TimerRequest;
    type Stream = BoxStream<'static, u64>;
    type Item = u64;

    fn handle_stream(&mut self, req: TimerRequest) -> Self::Stream {
        let mut mediator = self.0.clone();

        box_stream! { yx move =>
            for i in 0..req.0 {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                let sec = i + 1;
                yx.yield_one(sec);
                mediator.publish(SecondPassedEvent(sec)).expect("publish failed");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let mut mediator = DefaultMediator::builder()
        .add_stream_handler_deferred(TimerRequestHandler)
        // .add_stream_handler_fn_deferred(|req: TimerRequest, mut mediator: DefaultMediator| {
        //     box_stream! { yx move =>
        //         for i in 0..req.0 {
        //             tokio::time::sleep(Duration::from_millis(1000)).await;
        //             let s = i + 1;
        //             yx.yield_one(s);
        //             mediator.publish(SecondPassedEvent(s)).expect("publish failed");
        //         }
        //     }
        // })
        .subscribe_fn(|event: SecondPassedEvent| {
            println!("{} second passed", event.0);
        })
        .build();

    let mut stream = mediator.stream(TimerRequest(5)).unwrap();
    while stream.next().await.is_some() {}
}
