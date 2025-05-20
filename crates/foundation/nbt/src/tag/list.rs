use std::fmt;
use std::fmt::Formatter;
use std::{
    borrow::{Borrow, BorrowMut},
    ops::{Deref, DerefMut, Index, IndexMut},
};

use crate::settings::SnbtWriteOptions;
use crate::repr::{NbtReprError, NbtStructureError};

use super::{ListWithOptions, NbtTag};


/// The NBT tag list type which is essentially just a wrapper for a vec of NBT tags.
///
/// This type will implement both `Serialize` and `Deserialize` when the serde feature is enabled,
/// however this type should still be read and written with the utilities in the [`io`] module when
/// possible if speed is the main priority. See [`NbtTag`] for more details.
///
/// [`io`]: crate::io
/// [`NbtTag`]: crate::tag::NbtTag
#[repr(transparent)]
#[derive(Clone, PartialEq)]
pub struct NbtList(pub(crate) Vec<NbtTag>);

impl NbtList {
    /// Returns a new NBT tag list with an empty internal vec.
    #[inline]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Returns a mutable reference to the internal vector of this NBT list.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut Vec<NbtTag> {
        &mut self.0
    }

    /// Returns the internal vector of this NBT list.
    #[inline]
    pub fn into_inner(self) -> Vec<NbtTag> {
        self.0
    }

    /// Returns a new NBT tag list with the given initial capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Clones the data in the given list and converts it into an [`NbtList`].
    #[inline]
    pub fn clone_from<'a, T, L>(list: L) -> Self
    where
        T: Clone + Into<NbtTag> + 'a,
        L: IntoIterator<Item = &'a T>,
    {
        Self(
            list.into_iter()
                .map(|x| x.clone().into())
                .collect(),
        )
    }

    /// Iterates over references to each tag in this tag list
    #[inline]
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    /// Iterates over mutable references to each tag in this tag list
    #[inline]
    pub fn iter_mut(&mut self) -> <&mut Self as IntoIterator>::IntoIter {
        self.into_iter()
    }

    /// Iterates over this tag list, converting each tag reference into the specified type.
    #[inline]
    pub fn iter_map<'a, T: TryFrom<&'a NbtTag>>(
        &'a self,
    ) -> impl Iterator<Item = Result<T, <T as TryFrom<&'a NbtTag>>::Error>> + 'a {
        self.0.iter().map(|tag| T::try_from(tag))
    }

    /// Iterates over mutable references to the tags in this list, converting each tag reference
    /// into the specified type. See [`iter_map`](crate::tag::NbtList::iter_map) for usage
    /// details.
    #[inline]
    pub fn iter_mut_map<'a, T: TryFrom<&'a mut NbtTag>>(
        &'a mut self,
    ) -> impl Iterator<Item = Result<T, <T as TryFrom<&'a mut NbtTag>>::Error>> + 'a {
        self.0.iter_mut().map(|tag| T::try_from(tag))
    }

    /// Converts this tag list into a valid SNBT string. See `NbtTag::`[`to_snbt`] for details.
    ///
    /// [`to_snbt`]: crate::NbtTag::to_snbt
    #[inline]
    pub fn to_snbt(&self) -> String {
        format!("{self:?}")
    }

    /// Converts this tag list into a valid SNBT string with extra spacing for readability.
    /// See `NbtTag::`[`to_pretty_snbt`] for details.
    ///
    /// [`to_pretty_snbt`]: crate::NbtTag::to_pretty_snbt
    #[inline]
    pub fn to_pretty_snbt(&self) -> String {
        format!("{self:#?}")
    }

    /// Converts this tag list into a valid SNBT string.
    /// See `NbtTag::`[`to_snbt`] for details.
    ///
    /// [`to_snbt`]: crate::tag::NbtTag::to_snbt
    #[inline]
    pub fn to_snbt_with_options(&self, opts: SnbtWriteOptions) -> String {
        format!("{:?}", ListWithOptions::new(self, opts))
    }

    /// Converts this tag list into a valid SNBT string with extra spacing for readability.
    /// See `NbtTag::`[`to_pretty_snbt`] for details.
    ///
    /// [`to_pretty_snbt`]: crate::tag::NbtTag::to_pretty_snbt
    #[inline]
    pub fn to_pretty_snbt_with_options(&self, opts: SnbtWriteOptions) -> String {
        format!("{:#?}", ListWithOptions::new(self, opts))
    }

    /// Returns the length of this list.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if this tag list has a length of zero, false otherwise.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the value of the tag at the given index, or an error if the index is out of bounds
    /// or the the tag type does not match the type specified. This method should be used for
    /// obtaining primitives and shared references to lists and compounds.
    #[inline]
    pub fn get<'a, T>(&'a self, index: usize) -> Result<T, NbtReprError>
    where
        T: TryFrom<&'a NbtTag>,
        T::Error: Into<anyhow::Error>,
    {
        T::try_from(
            self.0
                .get(index)
                .ok_or_else(|| NbtStructureError::invalid_index(index, self.len()))?,
        )
        .map_err(NbtReprError::from_any)
    }

    /// Returns a mutable reference to the tag at the given index, or an error if the index is out
    /// of bounds or tag type does not match the type specified. This method should be used for
    /// obtaining mutable references to elements.
    #[inline]
    pub fn get_mut<'a, T>(&'a mut self, index: usize) -> Result<T, NbtReprError>
    where
        T: TryFrom<&'a mut NbtTag>,
        T::Error: Into<anyhow::Error>,
    {
        let len = self.len();
        T::try_from(
            self.0
                .get_mut(index)
                .ok_or_else(|| NbtStructureError::invalid_index(index, len))?,
        )
        .map_err(NbtReprError::from_any)
    }

    /// Returns a reference to the tag at the given index without any casting,
    /// or `None` if the index is out of bounds.
    #[inline]
    pub fn get_tag(&self, index: usize) -> Option<&NbtTag> {
        self.0.get(index)
    }

    /// Returns a mutable reference to the tag at the given index without any casting,
    /// or `None` if the index is out of bounds.
    #[inline]
    pub fn get_tag_mut(&mut self, index: usize) -> Option<&mut NbtTag> {
        self.0.get_mut(index)
    }

    /// While preserving the order of the `NbtList`, removes and returns the tag at the given index
    /// without any casting, or returns `None` if the index is out of bounds.
    #[inline]
    pub fn remove_tag(&mut self, index: usize) -> Option<NbtTag> {
        if index < self.0.len() {
            Some(self.0.remove(index))
        } else {
            None
        }
    }

    /// Removes and returns the tag at the given index without any casting,
    /// or returns `None` if the index is out of bounds. Does not preserve the order
    /// of the `NbtList`.
    #[inline]
    pub fn swap_remove_tag(&mut self, index: usize) -> Option<NbtTag> {
        if index < self.0.len() {
            Some(self.0.swap_remove(index))
        } else {
            None
        }
    }

    /// Pushes the given value to the back of the list after wrapping it in an `NbtTag`.
    #[inline]
    pub fn push<T: Into<NbtTag>>(&mut self, value: T) {
        self.0.push(value.into());
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
            return write!(f, "[]");
        }

        if f.alternate() {
            indent.push_str("    ");
            write!(f, "[\n")?;
        } else {
            write!(f, "[")?;
        }

        let last_index = self.len() - 1;
        for (index, element) in self.0.iter().enumerate() {
            if f.alternate() {
                write!(f, "{indent}")?;
            }

            // Conceptually, current_depth is the depth of this List itself;
            // its elements are one recursive tag deeper.
            element.recursively_format_snbt(indent, f, current_depth + 1, opts)?;

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
            write!(f, "\n{indent}]")
        } else {
            write!(f, "]")
        }
    }
}

impl Default for NbtList {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Into<NbtTag>> From<Vec<T>> for NbtList {
    #[inline]
    fn from(list: Vec<T>) -> Self {
        Self(list.into_iter().map(|x| x.into()).collect())
    }
}

impl IntoIterator for NbtList {
    type IntoIter = <Vec<NbtTag> as IntoIterator>::IntoIter;
    type Item = NbtTag;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a NbtList {
    type IntoIter = <&'a Vec<NbtTag> as IntoIterator>::IntoIter;
    type Item = &'a NbtTag;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut NbtList {
    type IntoIter = <&'a mut Vec<NbtTag> as IntoIterator>::IntoIter;
    type Item = &'a mut NbtTag;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl FromIterator<NbtTag> for NbtList {
    #[inline]
    fn from_iter<T: IntoIterator<Item = NbtTag>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl AsRef<[NbtTag]> for NbtList {
    #[inline]
    fn as_ref(&self) -> &[NbtTag] {
        &self.0
    }
}

impl AsMut<[NbtTag]> for NbtList {
    #[inline]
    fn as_mut(&mut self) -> &mut [NbtTag] {
        &mut self.0
    }
}

impl Borrow<[NbtTag]> for NbtList {
    #[inline]
    fn borrow(&self) -> &[NbtTag] {
        &self.0
    }
}

impl BorrowMut<[NbtTag]> for NbtList {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [NbtTag] {
        &mut self.0
    }
}

impl Deref for NbtList {
    type Target = [NbtTag];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NbtList {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Extend<NbtTag> for NbtList {
    #[inline]
    fn extend<T: IntoIterator<Item = NbtTag>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl Index<usize> for NbtList {
    type Output = NbtTag;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for NbtList {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}
