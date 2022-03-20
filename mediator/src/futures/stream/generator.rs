use crate::futures::Stream;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::{channel, Receiver};
use std::task::{Context, Poll};
use crate::futures::stream::Yielder;

/// Creates a stream using the provided function to generate the values.
pub fn generate<F, B, T>(builder: B) -> impl Stream<Item = T>
where
    B: FnMut(Yielder<T>) -> F,
    F: Future<Output = ()> + Unpin,
    T: Debug,
{
    StreamGenerator::new(builder)
}

struct StreamGenerator<T, B, F> {
    builder: B,
    yielder: Yielder<T>,
    receiver: Receiver<T>,
    future: Option<F>,
    done: bool,
}

impl<T, B, F> Unpin for StreamGenerator<T, B, F> {}

impl<T, B, F> StreamGenerator<T, B, F> {
    pub fn new(builder: B) -> Self {
        let (tx, receiver) = channel();
        let yielder = Yielder { tx };

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
    B: FnMut(Yielder<T>) -> F,
    F: Future<Output = ()> + Unpin,
    T: Debug,
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

        let future = future.get_or_insert_with(|| builder(yielder.clone()));

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
