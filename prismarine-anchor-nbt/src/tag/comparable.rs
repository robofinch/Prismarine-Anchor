use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};

#[cfg(feature = "float_cmp")]
use float_cmp::approx_eq;

use crate::raw::id_for_tag;
use super::NbtTag;


// ================================================================
//  ComparableNbtTag
// ================================================================

/// An `NbtTag` wrapper which implements `Eq`, `Ord`, and `Hash`.
///
/// The majority of issues in implementing these traits arise from `Float` and `Double` tags,
/// though `List` and `Compound` tags may not be as performant as others. There are no
/// recursion limits for comparisons.
///
/// Unlike the `PartialEq` implementation for `NbtList`, the order of elements in
/// a `List` tag does not matter for a `ComparableNbtTag`, matching Minecraft's behavior.
#[derive(Debug, Clone)]
pub struct ComparableNbtTag(pub NbtTag);

impl ComparableNbtTag {
    /// Create a new `ComparableNbtTag` wrapper around the provided tag.
    #[inline]
    pub fn new(tag: NbtTag) -> Self {
        Self(tag)
    }

    /// Check if two tags are equal, using the provided `FloatEquality` comparator.
    #[inline]
    pub fn equals<E: FloatEquality>(&self, other: &Self, equal: E) -> bool {
        // See "Main recursive functions" below
        tags_are_equal(&self.0, &other.0, equal)
    }

    /// Check if two tags are equal, using the provided `FloatEquality` comparator.
    #[inline]
    pub fn equals_tag<E: FloatEquality>(&self, other: &NbtTag, equal: E) -> bool {
        // See "Main recursive functions" below
        tags_are_equal(&self.0, other, equal)
    }

    /// Compare two tags, using the provided `FloatOrdering` comparator.
    #[inline]
    pub fn compare<O: FloatOrdering>(&self, other: &Self, compare: O) -> Ordering {
        // See "Main recursive functions" below
        compare_tags(&self.0, &other.0, compare)
    }

    /// Compare two tags, using the provided `FloatOrdering` comparator.
    #[inline]
    pub fn compare_tag<O: FloatOrdering>(&self, other: &NbtTag, compare: O) -> Ordering {
        // See "Main recursive functions" below
        compare_tags(&self.0, other, compare)
    }

    /// Get this exact tag from a `BTreeMap`, without concern for floating point issues.
    #[inline]
    pub fn get_from_map_exact<'a, T>(&self, map: &'a BTreeMap<Self, T>) -> Option<&'a T> {
        map.get(self)
    }

    /// Get this tag from a `BTreeMap`, with some leniency for the values of `Float`
    /// and `Double` tags' values. If multiple keys are within range of this tag,
    /// one of the two closest keys is chosen.
    #[cfg(feature = "float_cmp")]
    pub fn get_from_map_approx<'a, T>(&self, map: &'a BTreeMap<Self, T>) -> Option<&'a T> {
        // Try exact first, since it's cheaper.
        if let Some(value) = map.get(self) {
            return Some(value);
        }

        // If the tag couldn't possibly have a Float or Double tag in it, stop here.
        // The exact check would've caught anything.
        if !matches!(
            &self.0,
            NbtTag::Float(_) | NbtTag::Double(_) | NbtTag::List(_) | NbtTag::Compound(_)
        ) {
            return None;
        }

        const F32_FACTOR: f32 = 1.0 + f32::EPSILON * 3.;
        const F64_FACTOR: f64 = 1.0 + f64::EPSILON * 3.;

        fn lower_f32(f: f32) -> f32 {
            if f.is_nan() || f.is_infinite() {
                f
            } else if f.is_sign_positive() {
                f / F32_FACTOR
            } else {
                f * F32_FACTOR
            }
        }
        fn upper_f32(f: f32) -> f32 {
            if f.is_nan() || f.is_infinite() {
                f
            } else if f.is_sign_positive() {
                f * F32_FACTOR
            } else {
                f / F32_FACTOR
            }
        }
        fn lower_f64(f: f64) -> f64 {
            if f.is_nan() || f.is_infinite() {
                f
            } else if f.is_sign_positive() {
                f / F64_FACTOR
            } else {
                f * F64_FACTOR
            }
        }
        fn upper_f64(f: f64) -> f64 {
            if f.is_nan() || f.is_infinite() {
                f
            } else if f.is_sign_positive() {
                f * F64_FACTOR
            } else {
                f / F64_FACTOR
            }
        }

        // Compute a lower and upper approx bound for self. Is potentially very expensive.
        let (lower_bound, contains_float) = self.map_floats(lower_f32, lower_f64);

        if !contains_float {
            // The tag doesn't have a Float or Double tag in it
            return None;
        }

        let (upper_bound, _) = self.map_floats(upper_f32, upper_f64);

        let mut in_range = map.range(lower_bound..upper_bound);

        let Some(first_match) = in_range.next() else {
            // No key in the map was approximately equal to this tag
            return None;
        };

        if let Some(second_match) = in_range.next() {
            let mut all_matches = vec![first_match, second_match];
            all_matches.extend(in_range);

            match all_matches.binary_search_by_key(&self, |(key, _)| key) {
                Ok(found_index) => {
                    Some(all_matches[found_index].1)
                }
                Err(pos_index) => {
                    if pos_index >= all_matches.len() {
                        Some(all_matches[all_matches.len() - 1].1)
                    } else {
                        Some(all_matches[pos_index].1)
                    }
                }
            }
        } else {
            Some(first_match.1)
        }
    }

    #[cfg(feature = "float_cmp")]
    fn map_floats<F, D>(&self, map_float: F, map_double: D) -> (Self, bool)
    where
        F: Fn(f32) -> f32,
        D: Fn(f64) -> f64,
    {
        if let NbtTag::Float(f) = self.0 {
            return (Self(NbtTag::Float(map_float(f))), true)
        } else if let NbtTag::Double(d) = self.0 {
            return (Self(NbtTag::Double(map_double(d))), true)
        }

        let mut output: NbtTag = self.0.clone();
        let mut found_float = false;

        let mut map_queue = VecDeque::new();
        map_queue.push_back(&mut output);

        while let Some(tag) = map_queue.pop_front() {
            match tag {
                NbtTag::List(list) => {
                    for tag in list.iter_mut() {
                        match tag {
                            NbtTag::Float(f) => {
                                *f = map_float(*f);
                                found_float = true;
                            }
                            NbtTag::Double(d) => {
                                *d = map_double(*d);
                                found_float = true;
                            }
                            list @ NbtTag::List(_) => map_queue.push_back(list),
                            compound @ NbtTag::Compound(_) => map_queue.push_back(compound),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        (Self(output), found_float)
    }
}

impl From<NbtTag> for ComparableNbtTag {
    #[inline]
    fn from(tag: NbtTag) -> Self {
        Self(tag)
    }
}

impl From<ComparableNbtTag> for NbtTag {
    #[inline]
    fn from(tag: ComparableNbtTag) -> Self {
        tag.0
    }
}

impl PartialEq for ComparableNbtTag {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.equals(other, CompareExact)
    }
}

impl Eq for ComparableNbtTag {}

impl PartialOrd for ComparableNbtTag {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ComparableNbtTag {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // See "Main recursive functions" below
        compare_tags(&self.0, &other.0, CompareExact)
    }
}

// ================================================================
//  Main recursive functions
// ================================================================

fn tags_are_equal<E: FloatEquality>(
    tag: &NbtTag, other: &NbtTag, equal: E,
) -> bool {
    let mut compare_queue = VecDeque::new();

    compare_queue.push_back((tag, other));

    while let Some((self_tag, other_tag)) = compare_queue.pop_front() {
        match (self_tag, other_tag) {
            (NbtTag::Byte(n),   NbtTag::Byte(k))  if n == k => {}
            (NbtTag::Short(n),  NbtTag::Short(k)) if n == k => {}
            (NbtTag::Int(n),    NbtTag::Int(k))   if n == k => {}
            (NbtTag::Long(n),   NbtTag::Long(k))  if n == k => {}
            (NbtTag::Float(f),  NbtTag::Float(other))  if equal.equal_f32(*f, *other) => {}
            (NbtTag::Double(d), NbtTag::Double(other)) if equal.equal_f64(*d, *other) => {}
            (NbtTag::ByteArray(arr), NbtTag::ByteArray(other)) if arr == other => {}
            (NbtTag::IntArray(arr),  NbtTag::IntArray(other))  if arr == other => {}
            (NbtTag::LongArray(arr), NbtTag::LongArray(other)) if arr == other => {}
            (NbtTag::String(s), NbtTag::String(other)) if s == other => {}
            (NbtTag::List(list), NbtTag::List(other)) => {
                if list.len() != other.len() {
                    return false;
                }

                let mut unmatched_indices = Vec::with_capacity(list.len());
                for i in 0..list.len() {
                    unmatched_indices.push(i);
                }

                'outer: for value in list {
                    for i in 0..unmatched_indices.len() {
                        let other_val = &other[unmatched_indices[i]];
                        // Recursion isn't ideal, but this is a complex comparison.
                        if tags_are_equal(value, other_val, equal) {
                            unmatched_indices.swap_remove(i);
                            continue 'outer;
                        }
                    }
                    // If we got here, this value isn't equal to anything in the other list
                    return false;
                }

                // If we get here, everything in this list was matched to something in the
                // other list. The lists are approximately equal, up to order of elements.
            }
            (NbtTag::Compound(compound), NbtTag::Compound(other)) => {
                if compound.len() != other.len() {
                    return false;
                }

                for (key, value) in compound {
                    let Ok(other_val) = other.get(key) else {
                        return false
                    };

                    // Try to limit recursion.
                    compare_queue.push_back((value, other_val));
                }
            }
            _ => return false
        } // End of match
    } // End of loop

    true
}

fn compare_tags<O: FloatOrdering>(
    tag: &NbtTag, other: &NbtTag, compare: O,
) -> Ordering {
    match (tag, other) {
        (NbtTag::Byte(n),   NbtTag::Byte(k))  => n.cmp(k),
        (NbtTag::Short(n),  NbtTag::Short(k)) => n.cmp(k),
        (NbtTag::Int(n),    NbtTag::Int(k))   => n.cmp(k),
        (NbtTag::Long(n),   NbtTag::Long(k))  => n.cmp(k),
        (NbtTag::Float(f),  NbtTag::Float(other))  => compare.cmp_f32(*f, *other),
        (NbtTag::Double(d), NbtTag::Double(other)) => compare.cmp_f64(*d, *other),
        (NbtTag::ByteArray(arr), NbtTag::ByteArray(other)) => arr.cmp(other),
        (NbtTag::IntArray(arr),  NbtTag::IntArray(other))  => arr.cmp(other),
        (NbtTag::LongArray(arr), NbtTag::LongArray(other)) => arr.cmp(other),
        (NbtTag::String(s), NbtTag::String(other)) => s.cmp(other),
        (NbtTag::List(list), NbtTag::List(other)) => {
            // Sort lists of references to get a list-order-independent way of comparing lists

            let mut sorted_list: Vec<&_> = list.iter().collect();
            let mut other_list: Vec<&_> = other.iter().collect();
            sorted_list.sort_by(|t, o| compare_tags(t, o, compare));
            other_list.sort_by(|t, o| compare_tags(t, o, compare));

            for i in 0..sorted_list.len().min(other_list.len()) {
                let order = compare_tags(sorted_list[i], other_list[i], compare);
                if order != Ordering::Equal {
                    return order;
                }
            }

            // If we got here, one list is a equal to a prefix of the other.
            // Order longer lists as greater (IOW order them by length)
            sorted_list.len().cmp(&other_list.len())
        }
        (NbtTag::Compound(compound), NbtTag::Compound(other)) => {
            // As with lists, first make sorted vec's of keys to compare.
            let mut sorted_keys: Vec<&_> = compound.inner().keys().collect();
            let mut other_keys: Vec<&_> = other.inner().keys().collect();
            sorted_keys.sort();
            other_keys.sort();

            for i in 0..sorted_keys.len().min(other_keys.len()) {

                let key = sorted_keys[i];
                let other_key = other_keys[i];

                let key_order = key.cmp(other_key);
                if key_order != Ordering::Equal {
                    // If compound has keys "a", "b", "z" while other has keys "b", "c",
                    // this case would say compound is less than other.
                    // If compound has keys "a", "z" while other has keys "a", "c"
                    // with equal "a" values, then compound is greater.
                    return key_order;
                }

                let order = compare_tags(&compound[key], &other[other_key], compare);
                if order != Ordering::Equal {
                    return order;
                }
            }

            compound.len().cmp(&other.len())
        }
        _ => {
            let self_id  = id_for_tag(Some(tag));
            let other_id = id_for_tag(Some(other));
            self_id.cmp(&other_id)
        }
    }
}

// ================================================================
//  Comparison traits and structs
// ================================================================

/// Trait for checking whether two floats are equal. Should be cheap to copy.
pub trait FloatEquality: Copy {
    /// Returns `true` if the two floats are considered equal.
    fn equal_f32(self, value: f32, other: f32) -> bool;
    /// Returns `true` if the two floats are considered equal.
    fn equal_f64(self, value: f64, other: f64) -> bool;
}

/// Trait for comparing two floats. Should be cheap to copy. Not required to produce
/// a total order (e.g., `a = b`, `b = c`, and `a < c` could all be true).
pub trait FloatOrdering: Copy {
    /// Not required to produce a total order
    /// (e.g., `a = b`, `b = c`, and `a < c` could all be true).
    fn cmp_f32(self, value: f32, other: f32) -> Ordering;
    /// Not required to produce a total order
    /// (e.g., `a = b`, `b = c`, and `a < c` could all be true).
    fn cmp_f64(self, value: f64, other: f64) -> Ordering;
}

/// Provide a total order for `f32` and `f64`. All `NaN` values evaluate as equal
/// and greater than any other value (including `inf`).
#[derive(Debug, Clone, Copy)]
pub struct CompareExact;

impl FloatEquality for CompareExact {
    #[inline]
    fn equal_f32(self, value: f32, other: f32) -> bool {
        if value.is_nan() {
            other.is_nan()
        } else {
            value == other
        }
    }

    #[inline]
    fn equal_f64(self, value: f64, other: f64) -> bool {
        if value.is_nan() {
            other.is_nan()
        } else {
            value == other
        }
    }
}

impl FloatOrdering for CompareExact {
    fn cmp_f32(self, value: f32, other: f32) -> Ordering {
        if value.is_nan() {
            if other.is_nan() {
                Ordering::Equal
            } else {
                // value is NaN which is greater than anything else
                Ordering::Greater
            }
        } else if value < other {
            Ordering::Less
        } else if value > other {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }

    fn cmp_f64(self, value: f64, other: f64) -> Ordering {
        if value.is_nan() {
            if other.is_nan() {
                Ordering::Equal
            } else {
                // value is NaN which is greater than anything else
                Ordering::Greater
            }
        } else if value < other {
            Ordering::Less
        } else if value > other {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

/// Check whether two floats are approximately equal, and otherwise return
/// what [`CompareExact`] would.
#[cfg(feature = "float_cmp")]
#[derive(Debug, Clone, Copy)]
pub struct CompareApprox;

#[cfg(feature = "float_cmp")]
impl FloatEquality for CompareApprox {
    #[inline]
    fn equal_f32(self, value: f32, other: f32) -> bool {
        approx_eq!(f32, value, other, ulps = 5)
    }

    #[inline]
    fn equal_f64(self, value: f64, other: f64) -> bool {
        approx_eq!(f64, value, other, ulps = 5)
    }
}

#[cfg(feature = "float_cmp")]
impl FloatOrdering for CompareApprox {
    #[inline]
    fn cmp_f32(self, value: f32, other: f32) -> Ordering {
        if self.equal_f32(value, other) {
            Ordering::Equal
        } else {
            CompareExact.cmp_f32(value, other)
        }
    }

    #[inline]
    fn cmp_f64(self, value: f64, other: f64) -> Ordering {
        if self.equal_f64(value, other) {
            Ordering::Equal
        } else {
            CompareExact.cmp_f64(value, other)
        }
    }
}
