pub trait InspectNone {
    fn inspect_none<F: FnOnce()>(self, f: F) -> Self;
}

impl<T> InspectNone for Option<T> {
    fn inspect_none<F: FnOnce()>(self, f: F) -> Self {
        if let Some(value) = self {
            Some(value)
        } else {
            f();
            None
        }
    }
}
