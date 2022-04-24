use crate::futures::{Stream, StreamExt};
use std::sync::mpsc::Sender;
use std::sync::Mutex;

/// Generates values for a stream.
pub struct Yielder<T> {
    pub(crate) sender: Mutex<Sender<T>>,
}

impl<T> Clone for Yielder<T> {
    fn clone(&self) -> Self {
        let sender = self.sender.lock().expect("Unable to acquire lock").clone();
        Yielder {
            sender: Mutex::new(sender),
        }
    }
}

impl<T> Yielder<T> {
    pub fn new(sender: Sender<T>) -> Self {
        Self {
            sender: Mutex::new(sender),
        }
    }

    /// Sends a value to the stream.
    pub fn yield_one(&self, item: T) {
        self.sender
            .lock()
            .expect("unable to acquire lock")
            .send(item)
            .expect("Failed to send item");
    }

    /// Sends a list of values to the stream.
    pub fn yield_all<I>(&self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for item in iter {
            self.sender
                .lock()
                .expect("unable to acquire lock")
                .send(item)
                .expect("Failed to send item");
        }
    }

    /// Sends a stream of values to the stream.
    pub async fn yield_stream<S>(&self, mut stream: S)
    where
        S: Stream<Item = T> + Unpin,
    {
        while let Some(item) = stream.next().await {
            self.sender
                .lock()
                .expect("unable to acquire lock")
                .send(item)
                .expect("Failed to send item");
        }
    }
}
