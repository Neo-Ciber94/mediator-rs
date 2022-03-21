use mediator::futures::BoxStream;
use mediator::futures::StreamExt;
use mediator::{box_stream, DefaultMediator, Event, Mediator, StreamRequest};

struct TimerRequest(u64);
impl StreamRequest for TimerRequest {
    type Stream = BoxStream<'static, ()>;
    type Item = ();
}

#[derive(Clone)]
struct TickEvent;
impl Event for TickEvent {}

#[tokio::main]
async fn main() {
    let mut mediator = DefaultMediator::builder()
        .add_stream_handler_fn_deferred(|req: TimerRequest, mut m: DefaultMediator| {
            box_stream! { yx move =>
                for _ in 0..req.0 {
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    yx.yield_one(());
                    m.publish(TickEvent).expect("Can't publish event");
                }
            }
        })
        .subscribe_fn(move |_: TickEvent| {
            println!("tick");
        })
        .build();

    let mut timer = mediator.stream(TimerRequest(5)).unwrap();
    while let Some(_) = timer.next().await {}
}
