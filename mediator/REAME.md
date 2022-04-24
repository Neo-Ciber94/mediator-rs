# mediator-rs

![https://github.com/Neo-Ciber94/mediator-rs/blob/main/LICENSE](https://img.shields.io/badge/license-MIT-blue)
![https://docs.rs/mediator/latest/mediator/](https://img.shields.io/badge/docs-passing-brightgreen)
![https://crates.io/crates/mediator](https://img.shields.io/badge/crates.io-v0.1-brightgreen)
![https://github.com/Neo-Ciber94/mediator-rs/actions/workflows/ci.yml](https://github.com/Neo-Ciber94/mediator-rs/actions/workflows/ci.yml/badge.svg)

An implementation of the Mediator pattern in Rust
inspired in C# [MediatR](https://github.com/jbogard/MediatR).

## Mediator Pattern
https://en.wikipedia.org/wiki/Mediator_pattern

## Usage
```toml
[dependencies]
mediator = "0.2"
```

## Tests
To run the tests use the following command:
```bash
cargo test --all-features
```

## Examples

### Basic usage
```rust
use mediator::{DefaultMediator, Mediator, Request, Event, RequestHandler, EventHandler};

#[derive(Clone, Debug)]
enum Op {
 Add(f32, f32),
 Sub(f32, f32),
 Mul(f32, f32),
 Div(f32, f32),
}

struct MathRequest(Op);
impl Request<Option<f32>> for MathRequest {}

#[derive(Clone, Debug)]
struct MathEvent(Op, Option<f32>);
impl Event for MathEvent {}

struct MathRequestHandler(DefaultMediator);
impl RequestHandler<MathRequest, Option<f32>> for MathRequestHandler {
    fn handle(&mut self, req: MathRequest) -> Option<f32> {
        let result = match req.0 {
            Op::Add(a, b) => Some(a + b),
            Op::Sub(a, b) => Some(a - b),
            Op::Mul(a, b) => Some(a * b),
            Op::Div(a, b) => {
                if b == 0.0 { None } else { Some(a / b) }
            }
        };

        self.0.publish(MathEvent(req.0, result)).expect("publish failed");
        result
    }
}

fn main() {
    let mut mediator = DefaultMediator::builder()
        .add_handler_deferred(MathRequestHandler)
        .subscribe_fn(|event: MathEvent| {
           println!("{:?}", event);
         })
        .build();

    let result = mediator.send(MathRequest(Op::Add(1.0, 2.0))).expect("send failed");
    assert_eq!(result, Some(3.0));
}
```

### Async
Require the `async` feature enable.

```rust
use mediator::{DefaultAsyncMediator, AsyncMediator, Request};

struct MulRequest(f32, f32);
impl Request<f32> for MulRequest {}

#[tokio::main]
async fn main() {
    let mut mediator = DefaultAsyncMediator::builder()
        .add_handler(|req: MulRequest| async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            req.0 * req.1
        })
        .build();

    let result = mediator.send(MulRequest(2.0, 3.0)).await.expect("send failed");
    assert_eq!(result, 6.0);
}
```

### Streams
Require the `streams` feature enable.

```rust
use mediator::{StreamRequest, Event, DefaultAsyncMediator, AsyncMediator, box_stream};
use mediator::futures::{StreamExt, BoxStream};

struct CountdownRequest(u32);
impl StreamRequest for CountdownRequest {
    type Stream = BoxStream<'static, u32>;
    type Item = u32;
}

#[tokio::main]
async fn main() {
    let mut mediator = DefaultAsyncMediator::builder()
        .add_stream_handler_fn(|req: CountdownRequest| box_stream! { yx move =>
            let mut count = req.0;
            while count > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                yx.yield_one(count);
                count -= 1;
            }
         })
        .build();

    let mut stream = mediator.stream(CountdownRequest(3)).await.expect("stream failed");
    assert_eq!(stream.next().await.unwrap(), 3);
    assert_eq!(stream.next().await.unwrap(), 2);
    assert_eq!(stream.next().await.unwrap(), 1);
    assert_eq!(stream.next().await, None);
}
```

License: MIT
