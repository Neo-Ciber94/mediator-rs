use crate::futures::stream::Yielder;
use crate::futures::Stream;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::mpsc::{channel, Receiver};
use std::task::{Context, Poll};

/// Creates a stream that may return on error using the provided function to generate the values.
///
/// This function requires a lot of boilerplate, we recommend to use [`try_stream`] and [`box_try_stream`] macros instead.
pub fn generate_try_stream<F, B, T, E>(builder: B) -> impl Stream<Item = Result<T, E>>
where
    B: FnOnce(Yielder<T>) -> F,
    F: Future<Output = Result<(), E>> + Unpin,
{
    StreamGenerator::new(builder)
}

struct StreamGenerator<T, E, B, F> {
    builder: Option<B>,
    yielder: Option<Yielder<T>>,
    receiver: Receiver<T>,
    future: Option<F>,
    done: bool,
    error: Option<E>,
    _marker: PhantomData<E>,
}

impl<T, E, B, F> Unpin for StreamGenerator<T, E, B, F> {}

impl<T, E, B, F> StreamGenerator<T, E, B, F> {
    pub fn new(builder: B) -> Self {
        // We just use an arbitrary channel bound here
        let (sender, receiver) = channel();
        let builder = Some(builder);
        let yielder = Some(Yielder::new(sender));

        StreamGenerator {
            builder,
            yielder,
            receiver,
            future: None,
            error: None,
            done: false,
            _marker: PhantomData,
        }
    }
}

impl<T, E, B, F> Stream for StreamGenerator<T, E, B, F>
where
    B: FnOnce(Yielder<T>) -> F,
    F: Future<Output = Result<(), E>> + Unpin,
{
    type Item = Result<T, E>;

    #[allow(clippy::never_loop)]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let StreamGenerator {
            builder,
            yielder,
            receiver,
            future,
            done,
            error,
            ..
        } = self.get_mut();

        if *done {
            // Receive the rest of the items
            while let Ok(data) = receiver.try_recv() {
                return Poll::Ready(Some(Ok(data)));
            }

            if let Some(err) = error.take() {
                return Poll::Ready(Some(Err(err)));
            }

            return Poll::Ready(None);
        }

        let future = {
            match future {
                Some(f) => f,
                None => {
                    let builder = builder.take().expect("Builder was already called");
                    let yielder = yielder.take().unwrap();
                    future.get_or_insert(builder(yielder))
                }
            }
        };

        let poll = Pin::new(future).poll(cx);
        *done = poll.is_ready();

        match poll {
            Poll::Ready(result) => {
                if let Err(e) = result {
                    *error = Some(e);
                }

                // Receive the available items
                while let Ok(data) = receiver.try_recv() {
                    return Poll::Ready(Some(Ok(data)));
                }

                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
