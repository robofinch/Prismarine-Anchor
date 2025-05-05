pub trait InspectNone {
    fn inspect_none<F: FnOnce()>(self, f: F) -> Self;
}

impl<T> InspectNone for Option<T> {
    fn inspect_none<F: FnOnce()>(self, f: F) -> Self {
        match self {
            Some(value) => Some(value),
            None => {
                f();
                None
            }
        }
    }
}
