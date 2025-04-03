use std::slice;
use std::collections::BTreeMap;

use super::Map;

// Based on IndexMap


impl<T> FromIterator<(String, T)> for Map<T> {
    /// Create a `Map` from the sequence of key-value pairs in the iterable.
    fn from_iter<I: IntoIterator<Item = (String, T)>>(iterable: I) -> Self {
        let iter = iterable.into_iter();
        let mut map = Map::new();
        map.extend(iter);
        map
    }
}

impl<'a, T> IntoIterator for &'a Map<T> {
    type Item = (&'a String, &'a T);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Map<T> {
    type Item = (&'a String, &'a mut T);
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T> IntoIterator for Map<T> {
    type Item = (String, T);
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self)
    }
}

pub struct Iter<'a, T> {
    inner: &'a BTreeMap<String, T>,
    inserted: slice::Iter<'a, String>,
}

impl<T> Iter<'_, T> {
    pub fn new(map: &Map<T>) -> Iter<'_, T> {
        Iter {
            inner: &map.inner,
            inserted: map.inserted.iter(),
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (&'a String, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inserted.next().map(|key|
            (key, self.inner.get(key).expect("All inserted keys have values"))
        )
    }
}

pub struct IterMut<'a, T> {
    inner: &'a mut BTreeMap<String, T>,
    inserted: slice::Iter<'a, String>,
}

impl<T> IterMut<'_, T> {
    pub fn new(map: &mut Map<T>) -> IterMut<'_, T> {
        IterMut {
            inner: &mut map.inner,
            inserted: map.inserted.iter(),
        }
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (&'a String, &'a mut T);

    fn next(&mut self) -> Option<(&'a String, &'a mut T)> {

        if let Some(key) = self.inserted.next() {

            let value = self.inner.get_mut(key).expect("All inserted keys have values");

            // Safety: value is properly aligned, non-null, dereferenceable, and so on.
            // The one concern is aliasing: no other pointer to this T may exist.
            // This can only occur if `self.inserted.next()` returns the same key twice.
            // By the invariants of `Map`, this does not occur.
            let value = unsafe {
                &mut *(value as *mut T)
            };

            Some((key, value))
        } else {
            None
        }
    }
}

pub struct IntoIter<T> {
    inner: BTreeMap<String, T>,
    inserted: std::vec::IntoIter<String>,
}

impl<T> IntoIter<T> {
    pub fn new(map: Map<T>) -> IntoIter<T> {
        IntoIter {
            inner: map.inner,
            inserted: map.inserted.into_iter(),
        }
    }
}

impl<'a, T> Iterator for IntoIter<T> {
    type Item = (String, T);

    fn next(&mut self) -> Option<Self::Item> {

        let Some(key) = self.inserted.next() else {
            return None
        };

        let value = self.inner.remove(&key).expect("All inserted keys have values");

        self.inserted.next().map(|key|
            (key, value)
        )
    }
}
