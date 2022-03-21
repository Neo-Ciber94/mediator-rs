
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

