use std::fmt;
use std::{borrow::Borrow, fmt::Formatter, hash::Hash, ops::Index};

use crate::snbt;
use crate::snbt::SnbtError;
use crate::{
    repr::{NbtReprError, NbtStructureError},
    settings::{SnbtParseOptions, SnbtWriteOptions},
};

use super::{CompoundWithOptions, Map, NbtTag};


/// The NBT tag compound type which is essentially just a wrapper for a hash map of string keys
/// to tag values.
///
/// This type will implement both `Serialize` and `Deserialize` when the serde feature is enabled,
/// however this type should still be read and written with the utilities in the [`io`] module when
/// possible if speed is the main priority. See [`NbtTag`] for more details.
///
/// [`NbtTag`]: crate::NbtTag
/// [`io`]: crate::io
#[repr(transparent)]
#[derive(Clone, PartialEq)]
pub struct NbtCompound(pub(crate) Map<NbtTag>);

impl NbtCompound {
    /// Returns a new NBT tag compound with an empty internal hash map.
    #[inline]
    pub fn new() -> Self {
        Self(Map::new())
    }

    /// Returns a reference to the internal hash map of this compound.
    #[inline]
    pub fn inner(&self) -> &Map<NbtTag> {
        &self.0
    }

    /// Returns a mutable reference to the internal hash map of this compound.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut Map<NbtTag> {
        &mut self.0
    }

    /// Returns the internal hash map of this NBT compound.
    #[inline]
    pub fn into_inner(self) -> Map<NbtTag> {
        self.0
    }

    /// Returns a new NBT tag compound with the given initial capacity, unless the `comparable`
    /// feature is enabled, in which case no allocation is performed.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Map::with_capacity(capacity))
    }

    /// Clones the data in the given map and converts it into an
    /// [`NbtCompound`](crate::tag::NbtCompound).
    #[inline]
    pub fn clone_from<'a, K, V, M>(map: &'a M) -> Self
    where
        K: Clone + Into<String> + 'a,
        V: Clone + Into<NbtTag> + 'a,
        &'a M: IntoIterator<Item = (&'a K, &'a V)>,
    {
        Self(
            map.into_iter()
                .map(|(key, value)| (key.clone().into(), value.clone().into()))
                .collect(),
        )
    }

    /// Iterates over this tag compound, converting each tag reference into the specified type. Each
    /// key is paired with the result of the attempted conversion into the specified type. The
    /// iterator will not terminate even if some conversions fail.
    #[inline]
    pub fn iter_map<'a, T: TryFrom<&'a NbtTag>>(
        &'a self,
    ) -> impl Iterator<Item = (&'a str, Result<T, <T as TryFrom<&'a NbtTag>>::Error>)> + 'a {
        self.0
            .iter()
            .map(|(key, tag)| (key.as_str(), T::try_from(tag)))
    }

    /// Iterates over this tag compound, converting each mutable tag reference into the specified
    /// type. See [`iter_map`](crate::tag::NbtCompound::iter_map) for details.
    #[inline]
    pub fn iter_mut_map<'a, T: TryFrom<&'a mut NbtTag>>(
        &'a mut self,
    ) -> impl Iterator<Item = (&'a str, Result<T, <T as TryFrom<&'a mut NbtTag>>::Error>)> + 'a
    {
        self.0
            .iter_mut()
            .map(|(key, tag)| (key.as_str(), T::try_from(tag)))
    }

    /// Converts this tag compound into a valid SNBT string. See `NbtTag::`[`to_snbt`] for details.
    ///
    /// [`to_snbt`]: crate::tag::NbtTag::to_snbt
    #[inline]
    pub fn to_snbt(&self) -> String {
        format!("{self:?}")
    }

    /// Converts this tag compound into a valid SNBT string with extra spacing for readability.
    /// See `NbtTag::`[`to_pretty_snbt`] for details.
    ///
    /// [`to_pretty_snbt`]: crate::tag::NbtTag::to_pretty_snbt
    #[inline]
    pub fn to_pretty_snbt(&self) -> String {
        format!("{self:#?}")
    }

    /// Converts this tag compound into a valid SNBT string.
    /// See `NbtTag::`[`to_snbt`] for details.
    ///
    /// [`to_snbt`]: crate::tag::NbtTag::to_snbt
    #[inline]
    pub fn to_snbt_with_options(&self, opts: SnbtWriteOptions) -> String {
        format!("{:?}", CompoundWithOptions::new(self, opts))
    }

    /// Converts this tag compound into a valid SNBT string with extra spacing for readability.
    /// See `NbtTag::`[`to_pretty_snbt`] for details.
    ///
    /// [`to_pretty_snbt`]: crate::tag::NbtTag::to_pretty_snbt
    #[inline]
    pub fn to_pretty_snbt_with_options(&self, opts: SnbtWriteOptions) -> String {
        format!("{:#?}", CompoundWithOptions::new(self, opts))
    }

    /// Returns the number of tags in this compound.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the length of this compound is zero, false otherwise.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the value of the tag with the given name, or an error if no tag exists with the
    /// given name or specified type. This method should be used to obtain primitives as well as
    /// shared references to lists and compounds.
    #[inline]
    pub fn get<'a, 'b, K, T>(&'a self, name: &'b K) -> Result<T, NbtReprError>
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
        &'b K: Into<String>,
        T: TryFrom<&'a NbtTag>,
        T::Error: Into<anyhow::Error>,
    {
        T::try_from(
            self.0
                .get(name)
                .ok_or_else(|| NbtStructureError::missing_tag(name))?,
        )
        .map_err(NbtReprError::from_any)
    }

    /// Returns the value of the tag with the given name, or an error if no tag exists with the
    /// given name or specified type. This method should be used to obtain mutable references to
    /// lists and compounds.
    #[inline]
    pub fn get_mut<'a, 'b, K, T>(&'a mut self, name: &'b K) -> Result<T, NbtReprError>
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
        &'b K: Into<String>,
        T: TryFrom<&'a mut NbtTag>,
        T::Error: Into<anyhow::Error>,
    {
        T::try_from(
            self.0
                .get_mut(name)
                .ok_or_else(|| NbtStructureError::missing_tag(name))?,
        )
        .map_err(NbtReprError::from_any)
    }

    /// Returns whether or not this compound has a tag with the given name.
    #[inline]
    pub fn contains_key<'b, K>(&self, key: &'b K) -> bool
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
        &'b K: Into<String>,
    {
        self.0.contains_key(key)
    }

    /// Returns a reference to the tag with the given name without any casting,
    /// or `None` if no tag exists with the given name.
    #[inline]
    pub fn get_tag<'b, K>(&self, key: &'b K) -> Option<&NbtTag>
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
        &'b K: Into<String>,
    {
        self.0.get(key)
    }

    /// Returns a mutable reference to the tag with the given name without any casting,
    /// or `None` if no tag exists with the given name.
    #[inline]
    pub fn get_tag_mut<'b, K>(&mut self, key: &'b K) -> Option<&mut NbtTag>
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
        &'b K: Into<String>,
    {
        self.0.get_mut(key)
    }

    /// Removes and returns the tag with the given name without any casting,
    /// or `None` if no tag exists with the given name. If using the `preserve_order`
    /// feature, this method preserves the insertion order.
    #[cfg(feature = "preserve_order")]
    #[inline]
    pub fn remove_tag<'b, K>(&mut self, key: &'b K) -> Option<NbtTag>
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
        &'b K: Into<String>,
    {
        self.0.shift_remove(key)
    }

    /// Removes and returns the tag with the given name without any casting,
    /// or `None` if no tag exists with the given name. This method does not preserve the order
    /// of the map (if tracked with the `preserve_order` feature).
    #[inline]
    pub fn swap_remove_tag<'b, K>(&mut self, key: &'b K) -> Option<NbtTag>
    where
        String: Borrow<K>,
        K: Hash + Ord + Eq + ?Sized,
        &'b K: Into<String>,
    {
        #[cfg(feature = "preserve_order")]
        {
            self.0.swap_remove(key)
        }
        #[cfg(not(feature = "preserve_order"))]
        {
            self.0.remove(key)
        }
    }

    /// Adds the given value to this compound with the given name
    /// after wrapping that value in an `NbtTag`.
    #[inline]
    pub fn insert<K: Into<String>, T: Into<NbtTag>>(&mut self, name: K, value: T) {
        self.0.insert(name.into(), value.into());
    }

    #[inline]
    pub fn iter(&self) -> <&Map<NbtTag> as IntoIterator>::IntoIter {
        self.into_iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> <&mut Map<NbtTag> as IntoIterator>::IntoIter {
        self.into_iter()
    }

    /// Parses an NBT compound from SNBT
    #[inline]
    pub fn from_snbt(input: &str, opts: SnbtParseOptions) -> Result<Self, SnbtError> {
        snbt::parse_compound(input, opts)
    }

    /// Used in the `display_and_debug` macro in the tag module
    #[inline]
    pub(super) fn to_formatted_snbt(
        &self,
        f:    &mut Formatter<'_>,
        opts: SnbtWriteOptions,
    ) -> fmt::Result {
        self.recursively_format_snbt(&mut String::new(), f, 0, opts)
    }

    #[expect(clippy::write_with_newline)]
    pub(super) fn recursively_format_snbt(
        &self,
        indent:        &mut String,
        f:             &mut Formatter<'_>,
        current_depth: u32,
        opts:          SnbtWriteOptions,
    ) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "{{}}");
        }

        if f.alternate() {
            indent.push_str("    ");
            write!(f, "{{\n")?;
        } else {
            write!(f, "{{")?;
        }

        let last_index = self.len() - 1;
        for (index, (key, value)) in self.0.iter().enumerate() {
            let key = NbtTag::string_to_snbt(key, opts);

            if f.alternate() {
                write!(f, "{indent}{key}: ")?;
            } else {
                write!(f, "{key}:")?;
            }

            // Conceptually, current_depth is the depth of this Compound itself;
            // its elements are one recursive tag deeper.
            // Note that depth limits are checked in `NbtTag::recursively_format_snbt`
            value.recursively_format_snbt(indent, f, current_depth + 1, opts)?;

            if index != last_index {
                if f.alternate() {
                    write!(f, ",\n")?;
                } else {
                    write!(f, ",")?;
                }
            }
        }

        if f.alternate() {
            indent.truncate(indent.len() - 4);
            write!(f, "\n{indent}}}")
        } else {
            write!(f, "}}")
        }
    }
}

impl Default for NbtCompound {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for NbtCompound {
    type IntoIter = <Map<NbtTag> as IntoIterator>::IntoIter;
    type Item = <Map<NbtTag> as IntoIterator>::Item;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a NbtCompound {
    type IntoIter = <&'a Map<NbtTag> as IntoIterator>::IntoIter;
    type Item = <&'a Map<NbtTag> as IntoIterator>::Item;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut NbtCompound {
    type IntoIter = <&'a mut Map<NbtTag> as IntoIterator>::IntoIter;
    type Item = (&'a String, &'a mut NbtTag);

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl FromIterator<(String, NbtTag)> for NbtCompound {
    #[inline]
    fn from_iter<T: IntoIterator<Item = (String, NbtTag)>>(iter: T) -> Self {
        Self(Map::from_iter(iter))
    }
}

impl<Q: ?Sized> Index<&Q> for NbtCompound
where
    String: Borrow<Q>,
    Q: Eq + Hash + Ord,
{
    type Output = NbtTag;

    #[inline]
    fn index(&self, key: &Q) -> &NbtTag {
        &self.0[key]
    }
}

impl Extend<(String, NbtTag)> for NbtCompound {
    #[inline]
    fn extend<T: IntoIterator<Item = (String, NbtTag)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}
