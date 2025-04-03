//! Barebones map struct with the features used in this crate.
pub mod iter;


use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::ops::{Index, IndexMut};

use serde::{Serialize, Deserialize};

use self::iter::{Iter, IterMut};

// Based on IndexMap


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Map<T> {
    inner: BTreeMap<String, T>,
    inserted: Vec<String>,
}

impl<T> Map<T> {
    /// Create a new empty `Map`.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
            inserted: Vec::new(),
        }
    }

    /// Return the number of key-value pairs in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Return true if there are no key-value pairs in the map.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Remove all key-value pairs from the map.
    pub fn clear(&mut self) {
        self.inner.clear();
        self.inserted.clear();
    }

    /// Return an iterator over the key-value pairs in the map, in the order they were inserted.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(self)
    }

    /// Return an iterator over the key-value pairs in the map, in the order they were inserted.
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut::new(self)
    }

    /// Return an iterator over the keys of the map, in the order they were inserted.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.inserted.iter()
    }

    /// Consume self into an iterator over the keys of the map, in the order they were inserted.
    pub fn into_keys(self) -> impl Iterator<Item = String> {
        self.inserted.into_iter()
    }

    /// Return an iterator over the values of the map, in the order they were inserted.
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.inserted.iter().map(|key| self.inner.get(key).unwrap())
    }

    /// Consume self into an iterator over the values of the map, in the order they were inserted.
    pub fn into_values(mut self) -> impl Iterator<Item = T> {
        self.inserted.into_iter().map(move |key| self.inner.remove(&key).unwrap())
    }
}

impl<T> Map<T> {
    /// Insert a key-value pair in the map.
    ///
    /// If an equivalent key already exists in the map, the key remains and
    /// retains in its place in the order, its corresponding value is updated
    /// with `value`, and the older value is returned inside `Some(_)`.
    ///
    /// If no equivalent key existed in the map: the new key-value pair is
    /// inserted, last in order, and `None` is returned.
    pub fn insert(&mut self, key: String, value: T) -> Option<T> {
        match self.inner.insert(key.clone(), value) {
            Some(old) => {
                // Do not update self.inserted
                Some(old)
            }
            None => {
                self.inserted.push(key);
                None
            }
        }
    }
}

impl<T> Extend<(String, T)> for Map<T> {
    /// Extend the map with all key-value pairs in the iterable.
    ///
    /// This is equivalent to calling [`insert`][IndexMap::insert] for each of
    /// them in order, which means that for keys that already existed
    /// in the map, their value is updated but it keeps the existing order.
    ///
    /// New keys are inserted in the order they appear in the sequence. If
    /// equivalents of a key occur more than once, the last corresponding value
    /// prevails.
    fn extend<I: IntoIterator<Item = (String, T)>>(&mut self, iterable: I) {
        let iter = iterable.into_iter();
        iter.for_each(move |(k, v)| {
            self.insert(k, v);
        });
    }
}

impl<T> Map<T> {
    /// Returns a reference to the value corresponding to the supplied `key` if present.
    pub fn get<'a, Q: ?Sized>(&self, key: &'a Q) -> Option<&T>
    where
        String: Borrow<Q>,
        Q: Ord,
    {
        self.inner.get(key)
    }

    /// Returns a mutable reference to the value corresponding to the supplied `key` if present.
    pub fn get_mut<'a, Q: ?Sized>(&mut self, key: &'a Q) -> Option<&mut T>
    where
        String: Borrow<Q>,
        Q: Ord,
    {
        self.inner.get_mut(key)
    }

    /// Determines whether a key is in the Map
    pub fn contains_key<'a, Q>(&self, key: &'a Q) -> bool
    where
        String: Borrow<Q>,
        Q: Ord + Eq + ?Sized,
    {
        self.inner.contains_key(&key)
    }
}

/// Access [`Map`] values corresponding to a key.
impl<'a, T, Q: ?Sized> Index<&'a Q> for Map<T>
where
    String: Borrow<Q>,
    Q: Ord,
{
    type Output = T;

    /// Returns a reference to the value corresponding to the supplied `key`.
    ///
    /// ***Panics*** if `key` is not present in the map.
    fn index(&self, key: &'a Q) -> &T {
        self.get(key).expect("no entry found for key")
    }
}

/// Access [`Map`] values corresponding to a key.
impl<'a, T, Q: ?Sized> IndexMut<&'a Q> for Map<T>
where
    String: Borrow<Q>,
    Q: Ord,
{
    // type Output = T;

    /// Returns a mutable reference to the value corresponding to the supplied `key`.
    ///
    /// ***Panics*** if `key` is not present in the map.
    fn index_mut(&mut self, key: &'a Q) -> &mut T {
        self.get_mut(key).expect("no entry found for key")
    }
}
