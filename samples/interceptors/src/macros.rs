#![allow(unused)]

#[doc(hidden)]
pub struct Finally<F>
where
    F: FnOnce(),
{
    pub cleanup: Option<F>,
}

impl<F> Drop for Finally<F>
where
    F: FnOnce(),
{
    fn drop(&mut self) {
        match self.cleanup.take() {
            Some(cleanup) => cleanup(),
            None => (),
        }
    }
}

#[macro_export]
macro_rules! tryfinally {
    (try $try_block:block finally $finally_block:block) => {{
        let _ = $crate::macros::Finally {
            cleanup: Some(|| $finally_block),
        };
        $try_block
    }};
}
