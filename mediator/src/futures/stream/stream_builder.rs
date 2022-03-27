use std::pin::Pin;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::task::{Context, Poll};
use tokio_stream::Stream;

/// A sender of items for an `StreamBuilder`.
pub struct StreamSender<T> {
    sender: Sender<Option<T>>,
}

impl<T> StreamSender<T> {
    /// Sends an item to the stream.
    pub fn send(&self, item: T) {
        self.sender.send(Some(item)).expect("send failed");
    }

    fn finish(&self) {
        self.sender.send(None).expect("send failed");
    }
}

impl<T> Clone for StreamSender<T> {
    fn clone(&self) -> Self {
        StreamSender {
            sender: self.sender.clone(),
        }
    }
}

/// A stream builder for creating `Stream`s.
pub struct StreamBuilder<T> {
    sender: StreamSender<T>,
    receiver: Receiver<Option<T>>,
}

impl<T> StreamBuilder<T> {
    /// Creates a `Stream` using the items sent to the stream builder.
    ///
    /// Any value sent before calling `build` will be ignored.
    pub fn build(self) -> impl Stream<Item = T> {
        self.sender.finish();

        StreamImpl {
            receiver: self.receiver,
            done: false,
        }
    }
}

struct StreamImpl<T> {
    receiver: Receiver<Option<T>>,
    done: bool,
}

impl<T> Stream for StreamImpl<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let StreamImpl { receiver, done } = self.get_mut();

        if *done {
            return Poll::Ready(None);
        }

        match receiver.try_recv() {
            Ok(Some(val)) => Poll::Ready(Some(val)),
            Ok(None) => {
                *done = true;
                Poll::Ready(None)
            }
            Err(TryRecvError::Empty) => Poll::Pending,
            Err(TryRecvError::Disconnected) => {
                panic!("Stream builder was disconnected");
            }
        }
    }
}

/// Creates a builder for a `Stream` with a `sender` and `builder`.
///
/// # Example
/// ```
/// use mediator::futures::stream::stream_builder;
/// use mediator::futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() {
///     let (sender, builder) = stream_builder::<i32>();
///     sender.send(1);
///     sender.send(2);
///     tokio::time::sleep(std::time::Duration::from_millis(100)).await;
///     sender.send(3);
///
///     let mut stream = builder.build();
///
///     assert_eq!(stream.next().await, Some(1));
///     assert_eq!(stream.next().await, Some(2));
///     assert_eq!(stream.next().await, Some(3));
///     assert_eq!(stream.next().await, None);
/// }
/// ```
pub fn stream_builder<T>() -> (StreamSender<T>, StreamBuilder<T>) {
    let (sender, receiver) = std::sync::mpsc::channel();
    let builder = StreamBuilder {
        sender: StreamSender {
            sender: sender.clone(),
        },
        receiver,
    };

    (StreamSender { sender }, builder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_stream_builder() {
        let (sender, builder) = stream_builder::<i32>();

        sender.send(1);
        sender.send(2);
        sender.send(3);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        sender.send(4);

        let mut stream = builder.build();
        sender.send(5);

        assert_eq!(stream.next().await, Some(1));
        assert_eq!(stream.next().await, Some(2));
        assert_eq!(stream.next().await, Some(3));
        assert_eq!(stream.next().await, Some(4));
        assert_eq!(stream.next().await, None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_stream_async_builder() {
        let (sender, builder) = stream_builder::<i32>();
        sender.send(1);
        sender.send(2);

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            sender.send(3);
            sender.send(4);
        }).await.unwrap();

        let mut stream = builder.build();

        assert_eq!(stream.next().await, Some(1));
        assert_eq!(stream.next().await, Some(2));
        assert_eq!(stream.next().await, Some(3));
        assert_eq!(stream.next().await, Some(4));
        assert_eq!(stream.next().await, None);
    }
}
