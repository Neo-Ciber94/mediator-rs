use std::sync::mpsc::Sender;
use crate::futures::{Stream, StreamExt};

/// Generates values for a stream.
pub struct Yielder<T> {
    pub(crate) tx: Sender<T>,
}

impl<T> Yielder<T> {
    /// Sends a value to the stream.
    pub fn yield_one(&self, item: T) {
        self.tx.send(item).expect("Failed to send item");
    }

    /// Sends a list of values to the stream.
    pub fn yield_all<I>(&self, iter: I)
        where
            I: IntoIterator<Item = T>,
    {
        for item in iter {
            self.tx.send(item).expect("Failed to send item");
        }
    }

    /// Sends a stream of values to the stream.
    pub async fn yield_stream<S>(&self, mut stream: S)
        where
            S: Stream<Item = T> + Unpin,
    {
        while let Some(item) = stream.next().await {
            self.tx.send(item).expect("Failed to send item");
        }
    }
}

impl<T> Clone for Yielder<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}