#[macro_export]
macro_rules! stream {
    (|$yielder:ident| { $($tt:tt)*}) => {{
        $crate::futures::stream::generate(|$yielder| Box::pin(async {
            $($tt)*
        }))
    }};

    (|$yielder:ident| move { $($tt:tt)*}) => {{
        $crate::futures::stream::generate(move |$yielder| Box::pin(async move {
            $($tt)*
        }))
    }};
}

#[macro_export]
macro_rules! box_stream {
    (|$yielder:ident| { $($tt:tt)*}) => {{
        Box::pin($crate::futures::stream::generate(|$yielder| Box::pin(async {
            $($tt)*
        })))
    } as std::pin::Pin<Box<dyn $crate::futures::Stream<Item = _> + Send + '_>>};

    (|$yielder:ident| move { $($tt:tt)*}) => {{
        Box::pin($crate::futures::stream::generate(move |$yielder| Box::pin(async move {
            $($tt)*
        })))
    } as std::pin::Pin<Box<dyn $crate::futures::Stream<Item = _> + Send + '_>>};
}
