use crate::futures::stream::Yielder;
use crate::futures::Stream;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::{channel, Receiver};
use std::task::{Context, Poll};

/// Creates a stream using the provided function to generate the values.
///
/// This function requires a lot of boilerplate, we recommend to use [`stream`] and [`box_stream`] macros instead.
///
/// # Example
/// ```rust
/// use mediator::futures::stream::generate_stream;
/// use mediator::futures::StreamExt;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     let mut stream = generate_stream(|yx| Box::pin(async move {
///         // Sends single values
///         yx.yield_one(1);
///         yx.yield_one(2);
///         yx.yield_one(3);
///
///         tokio::time::sleep(Duration::from_secs(1)).await;
///
///         // Sends multiple values
///         yx.yield_all(vec![4, 5]);
///
///         // Sends other stream
///         yx.yield_stream(generate_stream(|yx2| Box::pin(async move {
///             yx2.yield_one(6)
///         }))).await;
///     }));
///
///     assert_eq!(stream.next().await, Some(1));
///     assert_eq!(stream.next().await, Some(2));
///     assert_eq!(stream.next().await, Some(3));
///     assert_eq!(stream.next().await, Some(4));
///     assert_eq!(stream.next().await, Some(5));
///     assert_eq!(stream.next().await, Some(6));
///     assert_eq!(stream.next().await, None);
/// }
/// ```
///
/// [`stream`]: crate::stream
/// [`box_stream`]: crate::box_stream
pub fn generate_stream<F, B, T>(builder: B) -> impl Stream<Item = T>
where
    B: FnOnce(Yielder<T>) -> F,
    F: Future<Output = ()> + Unpin,
{
    StreamGenerator::new(builder)
}

struct StreamGenerator<T, B, F> {
    builder: Option<B>,
    yielder: Yielder<T>,
    receiver: Receiver<T>,
    future: Option<F>,
    done: bool,
}

impl<T, B, F> Unpin for StreamGenerator<T, B, F> {}

impl<T, B, F> StreamGenerator<T, B, F> {
    pub fn new(builder: B) -> Self {
        let (sender, receiver) = channel();
        let builder = Some(builder);
        let yielder = Yielder { sender };

        StreamGenerator {
            builder,
            yielder,
            receiver,
            future: None,
            done: false,
        }
    }
}

impl<T, B, F> Stream for StreamGenerator<T, B, F>
where
    B: FnOnce(Yielder<T>) -> F,
    F: Future<Output = ()> + Unpin,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let StreamGenerator {
            builder,
            yielder,
            receiver,
            future,
            done,
        } = self.get_mut();

        if *done {
            // Receive the rest of the items
            while let Ok(data) = receiver.try_recv() {
                return Poll::Ready(Some(data));
            }

            return Poll::Ready(None);
        }

        let future = {
            match future {
                Some(f) => f,
                None => {
                    let builder = builder.take().unwrap();
                    future.get_or_insert(builder(yielder.clone()))
                }
            }
        };

        let poll = Pin::new(future).poll(cx);
        *done = poll.is_ready();

        // Receive the available items
        while let Ok(data) = receiver.try_recv() {
            return Poll::Ready(Some(data));
        }

        match poll {
            Poll::Ready(()) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
