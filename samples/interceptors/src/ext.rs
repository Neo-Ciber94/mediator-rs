pub trait ObjectExt {
    fn also<F: FnOnce(Self) -> Self>(self, f: F) -> Self
    where
        Self: Sized;

    fn also_if<F: FnOnce(Self) -> Self>(self, condition: bool, f: F) -> Self
        where
            Self: Sized;
}

impl<T> ObjectExt for T {
    fn also<F: FnOnce(Self) -> Self>(self, f: F) -> Self
    where
        Self: Sized,
    {
        f(self)
    }

    fn also_if<F: FnOnce(Self) -> Self>(self, condition: bool, f: F) -> Self
    where
        Self: Sized,
    {
        if condition {
            self.also(f)
        } else {
            self
        }
    }
}
