/// Constructs a `impl Stream<Item=T>` from a generator.
///
/// # Examples
/// ```rust
/// use mediator::stream;
/// use mediator::futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() {
///     let mut stream = stream!(|yx| move {
///         for i in 0..10 {
///             yx.yield_one(i);
///         }
///     });
///
///     while let Some(item) = stream.next().await {
///         println!("{}", item);
///     }
/// }
/// ```
#[macro_export]
macro_rules! stream {
    (|$yielder:ident| { $($tt:tt)*}) => {{
        $crate::futures::stream::generate_stream(|$yielder| Box::pin(async move {
            $($tt)*
        }))
    }};

    (|$yielder:ident| move { $($tt:tt)*}) => {{
        $crate::futures::stream::generate_stream(move |$yielder| Box::pin(async move {
            $($tt)*
        }))
    }};

    ($yielder:ident => $($tt:tt)* ) => {{
        $crate::stream!(|$yielder| { $($tt)* })
    }};

    ($yielder:ident move => $($tt:tt)* ) => {{
       $crate::stream!(|$yielder| move { $($tt)* })
    }};
}

/// Constructs a `Pin<Box<dyn Stream<Item=T> + Send>>` stream from a generator.
///
/// # Examples
/// ```rust
/// use mediator::box_stream;
/// use mediator::futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() {
///     let mut stream = box_stream!(|yx| move {
///         for i in 0..10 {
///             yx.yield_one(i);
///         }
///     });
///
///     while let Some(item) = stream.next().await {
///         println!("{}", item);
///     }
/// }
/// ```
#[macro_export]
macro_rules! box_stream {
    (|$yielder:ident| { $($tt:tt)*}) => {{
        Box::pin($crate::futures::stream::generate_stream(|$yielder| Box::pin(async move {
            $($tt)*
        })))
    } as std::pin::Pin<Box<dyn $crate::futures::Stream<Item = _> + Send + '_>>};

    (|$yielder:ident| move { $($tt:tt)*}) => {{
        Box::pin($crate::futures::stream::generate_stream(move |$yielder| Box::pin(async move {
            $($tt)*
        })))
    } as std::pin::Pin<Box<dyn $crate::futures::Stream<Item = _> + Send + '_>>};

    ($yielder:ident => $($tt:tt)* ) => {{
        $crate::box_stream!(|$yielder| { $($tt)* })
    }};

    ($yielder:ident move => $($tt:tt)* ) => {{
       $crate::box_stream!(|$yielder| move { $($tt)* })
    }};
}

/// Constructs a `impl Stream<Item=Result<T, E>>` from a generator.
///
/// # Example
/// ```rust
/// use mediator::try_stream;
/// use mediator::futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() {
///     let mut stream = try_stream!{ yx move =>
///         yx.yield_one(1);
///         yx.yield_one(2);
///
///         return Err("Something went wrong");
///     };
///
///     assert_eq!(Some(Ok(1)), stream.next().await);
///     assert_eq!(Some(Ok(2)), stream.next().await);
///     assert_eq!(Some(Err("Something went wrong")), stream.next().await);
/// }
/// ```
#[macro_export]
macro_rules! try_stream {
    (|$yielder:ident| { $($tt:tt)*}) => {{
        #[allow(unreachable_code)]
        $crate::futures::stream::generate_try_stream(|$yielder| Box::pin(async move {
            let _ = { $($tt)* };
            Ok(())
        }))
    }};

    (|$yielder:ident| move { $($tt:tt)*}) => {{
        #[allow(unreachable_code)]
        $crate::futures::stream::generate_try_stream(move |$yielder| Box::pin(async move {
            let _ = { $($tt)* };
            Ok(())
        }))
    }};

    ($yielder:ident => $($tt:tt)* ) => {{
        $crate::try_stream!(|$yielder| { $($tt)* })
    }};

    ($yielder:ident move => $($tt:tt)* ) => {{
       $crate::try_stream!(|$yielder| move { $($tt)* })
    }};
}

/// Constructs a `Pin<Box<dyn Stream<Item=Result<T, E>> + Send>>` stream from a generator.
///
/// # Example
/// ```rust
/// use mediator::box_try_stream;
/// use mediator::futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() {
///     let mut stream = box_try_stream!{ yx move =>
///         yx.yield_one(1);
///         yx.yield_one(2);
///
///         return Err("Something went wrong");
///     };
///
///     assert_eq!(Some(Ok(1)), stream.next().await);
///     assert_eq!(Some(Ok(2)), stream.next().await);
///     assert_eq!(Some(Err("Something went wrong")), stream.next().await);
/// }
/// ```
#[macro_export]
macro_rules! box_try_stream {
    (|$yielder:ident| { $($tt:tt)*}) => {{
        #[allow(unreachable_code)]
        Box::pin($crate::futures::stream::generate_try_stream(|$yielder| Box::pin(async move {
            let _ = { $($tt)* };
            Ok(())
        })))
    } as std::pin::Pin<Box<dyn $crate::futures::Stream<Item = Result<_, _>> + Send + '_>>};

    (|$yielder:ident| move { $($tt:tt)*}) => {{
        #[allow(unreachable_code)]
        Box::pin($crate::futures::stream::generate_try_stream(move |$yielder| Box::pin(async move {
            let _ = { $($tt)* };
            Ok(())
        })))
    } as std::pin::Pin<Box<dyn $crate::futures::Stream<Item = Result<_, _>> + Send + '_>>};

    ($yielder:ident => $($tt:tt)* ) => {{
        $crate::box_try_stream!(|$yielder| { $($tt)* })
    }};

    ($yielder:ident move => $($tt:tt)* ) => {{
       $crate::box_try_stream!(|$yielder| move { $($tt)* })
    }};
}

#[cfg(test)]
mod tests {
    use tokio_stream::StreamExt;

    #[tokio::test(flavor = "multi_thread")]
    async fn stream_test() {
        let mut stream = stream!(|yx| {
            yx.yield_one(1);
            yx.yield_one(2);
            yx.yield_one(3);

            yx.yield_all(vec![4, 5, 6]);

            yx.yield_stream(stream!(|yx2| {
                yx2.yield_one(7);
                yx2.yield_one(8);
                yx2.yield_one(9);
                yx2.yield_one(10);
            }))
            .await;
        });

        for i in 1..=10 {
            assert_eq!(stream.next().await, Some(i));
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn box_stream_test() {
        let mut stream = box_stream!(|yx| {
            yx.yield_one(1);
            yx.yield_one(2);
            yx.yield_one(3);

            yx.yield_all(vec![4, 5, 6]);

            yx.yield_stream(box_stream!(|yx2| {
                yx2.yield_one(7);
                yx2.yield_one(8);
                yx2.yield_one(9);
                yx2.yield_one(10);
            }))
            .await;
        });

        for i in 1..=10 {
            assert_eq!(stream.next().await, Some(i));
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn box_try_stream_test() {
        let mut stream = box_try_stream!(|yx| {
            yx.yield_one(1);
            yx.yield_one(2);
            yx.yield_one(3);

            return Err("Something went wrong");
        });

        assert_eq!(Ok(1), stream.next().await.unwrap());
        assert_eq!(Ok(2), stream.next().await.unwrap());
        assert_eq!(Ok(3), stream.next().await.unwrap());
        assert_eq!(Err("Something went wrong"), stream.next().await.unwrap());
        assert!(stream.next().await.is_none());
    }
}
