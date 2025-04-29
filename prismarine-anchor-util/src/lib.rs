//! Utilities without functionality specific to Prismarine Anchor, which don't particularly
//! fit in other crates in this project.

use std::sync::{Mutex, MutexGuard};


/// Converts two hexadecimal digits into a `u8`. Returns `None` if either character
/// is not a valid hexadecimal digit (`0-9`, `a-f`, `A-F`).
///
/// Note that the first character is the most significant nibble of the returned `u8`,
/// and the second character is the least significant nibble.
///
/// # Examples:
/// ```
/// # use prismarine_anchor_util::chars_to_u8;
/// assert_eq!(chars_to_u8(['f', 'f']), Some(255));
/// assert_eq!(chars_to_u8(['2', 'a']), Some(42));
/// assert_eq!(chars_to_u8(['x', '0']), None);
/// ```
#[inline]
pub fn chars_to_u8(chars: [char; 2]) -> Option<u8> {
    let nibbles = [
        // The u32's are actually in range of u8, because they're hex digits
        chars[0].to_digit(16)? as u8,
        chars[1].to_digit(16)? as u8,
    ];

    Some((nibbles[0] << 4) + nibbles[1])
}

/// Converts four hexadecimal digits into a `u16`. Returns `None` if any character
/// is not a valid hexadecimal digit (`0-9`, `a-f`, `A-F`).
///
/// Note that the first character is the most significant nibble of the returned `u16`,
/// and the last character is the least significant nibble.
///
/// # Examples:
/// ```
/// # use prismarine_anchor_util::chars_to_u16;
/// assert_eq!(chars_to_u16(['0', '0', 'f', 'f']), Some(255));
/// assert_eq!(chars_to_u16(['1', '1', 'a', 'a']), Some(4522));
/// assert_eq!(chars_to_u16(['0', '_', '0', '0']), None);
/// ```
#[inline]
pub fn chars_to_u16(chars: [char; 4]) -> Option<u16> {
    let nibbles = chars.map(|c| c.to_digit(16));

    let mut sum: u32 = 0;
    for nibble in nibbles {
        sum = (sum << 4) + nibble?;
    }

    // The sum is actually in range of u16, because there are four 4-bit nibbles.
    Some(sum as u16)
}

/// Converts eight hexadecimal digits into a `u32`. Returns `None` if any character
/// is not a valid hexadecimal digit (`0-9`, `a-f`, `A-F`).
///
/// Note that the first character is the most significant nibble of the returned `u32`,
/// and the last character is the least significant nibble.
///
/// # Examples:
/// ```
/// # use prismarine_anchor_util::chars_to_u32;
/// assert_eq!(chars_to_u32(['0', '0', '0', '0', '0', '0', '0', 'f']), Some(15));
/// assert_eq!(chars_to_u32(['1', '2', '3', '4', 'a', 'b', 'c', 'd']), Some(305_441_741));
/// assert_eq!(chars_to_u32(['0', '0', '0', '0', '_', '0', '0', '0']), None);
/// ```
#[inline]
pub fn chars_to_u32(chars: [char; 8]) -> Option<u32> {
    let nibbles = chars.map(|c| c.to_digit(16));

    let mut sum: u32 = 0;
    for nibble in nibbles {
        sum = (sum << 4) + nibble?;
    }

    Some(sum)
}

/// Converts eight hexadecimal digits into a `u32`. Returns `None` if any character
/// is not a valid hexadecimal digit (`0-9`, `a-f`, `A-F`).
///
/// Note that the first character (of the first array) is the most significant nibble of the
/// returned `u32`, and the last character (of the second array) is the least significant nibble.
///
/// # Examples:
/// ```
/// # use prismarine_anchor_util::pair_to_u32;
/// assert_eq!(pair_to_u32((['0', '0', '0', '0'], ['0', '0', '0', 'f'])), Some(15));
/// assert_eq!(pair_to_u32((['1', '2', '3', '4'], ['a', 'b', 'c', 'd'])), Some(305_441_741));
/// assert_eq!(pair_to_u32((['0', '0', '0', '0'], ['_', '0', '0', '0'])), None);
/// ```
#[inline]
pub fn pair_to_u32(chars: ([char; 4], [char; 4])) -> Option<u32> {
    let upper = chars.0.map(|c| c.to_digit(16));
    let lower = chars.1.map(|c| c.to_digit(16));

    let mut sum: u32 = 0;

    for nibble in upper {
        sum = (sum << 4) + nibble?;
    }
    for nibble in lower {
        sum = (sum << 4) + nibble?;
    }

    Some(sum)
}

/// Compile-time checked version of code like `slice_or_vec[4..8].try_into().unwrap()`
/// for converting a slice into an array.
///
/// Effectively, this converts `slice[START..END]` into a `[T; N]` array. The function confirms
/// at compile time that `START + N == END` (note that if `START <= END`, this constraint is
/// the same as `N == END - START`). Throws a compile-time error if this requirement is not met,
/// or if `START + N` overflows a usize.
///
/// # Panics
/// Panics if any index in the `START..END` range is out-of-bounds for the provided slice.
///
/// # Examples
/// ```
/// # use prismarine_anchor_util::slice_to_array;
/// let data: [u8; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];
/// assert_eq!(
///     slice_to_array::<0, 4, _, 4>(data.as_slice()),
///     [0, 1, 2, 3],
/// );
/// assert_eq!(
///     slice_to_array::<4, 9, _, 5>(data.as_slice()),
///     [4, 5, 6, 7, 8],
/// );
///
/// fn fn_that_only_gets_a_slice(bytes: &[u8]) -> Option<[u8; 4]> {
///     if bytes.len() < 5 {
///         None
///     } else {
///         Some(slice_to_array::<1, 5, _, 4>(bytes))
///     }
/// }
///
/// assert_eq!(
///     fn_that_only_gets_a_slice(data.as_slice()),
///     Some([1, 2, 3, 4]),
/// );
///
/// let data_vec: Vec<u8> = vec![4, 2];
/// let data_arr: [u8; 2] = slice_to_array::<0, 2, _, 2>(&data_vec);
/// assert_eq!(data_arr, [4, 2]);
/// ```
#[inline]
pub fn slice_to_array<const START: usize, const END: usize, T: Copy, const N: usize>(
    slice: &[T],
) -> [T; N] {
    *slice_to_array_ref::<START, END, T, N>(slice)
}

/// Compile-time checked version of code like `slice_or_vec[4..8].try_into().unwrap()`
/// for converting a slice into a reference to an array.
///
/// Effectively, this converts `slice[START..END]` into a reference to a`[T; N]` array.
/// The function confirms at compile time that `START + N == END`
/// (note that if `START <= END`, this constraint is the same as `N == END - START`).
/// Throws a compile-time error if this requirement is not met,
/// or if `START + N` overflows a usize.
///
/// # Panics
/// Panics if any index in the `START..END` range is out-of-bounds for the provided slice.
///
/// # Examples
/// ```
/// # use prismarine_anchor_util::slice_to_array_ref;
/// let data: [u8; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];
/// assert_eq!(
///     slice_to_array_ref::<0, 4, _, 4>(data.as_slice()),
///     &[0, 1, 2, 3],
/// );
/// assert_eq!(
///     slice_to_array_ref::<4, 9, _, 5>(data.as_slice()),
///     &[4, 5, 6, 7, 8],
/// );
///
/// fn fn_that_only_gets_a_slice(bytes: &[u8]) -> Option<&[u8; 4]> {
///     if bytes.len() < 5 {
///         None
///     } else {
///         Some(slice_to_array_ref::<1, 5, _, 4>(bytes))
///     }
/// }
///
/// assert_eq!(
///     fn_that_only_gets_a_slice(data.as_slice()),
///     Some(&[1, 2, 3, 4]),
/// );
///
/// let data_vec: Vec<u8> = vec![4, 2];
/// let data_arr: &[u8; 2] = slice_to_array_ref::<0, 2, _, 2>(&data_vec);
/// assert_eq!(data_arr, &[4, 2]);
/// ```
#[inline]
pub fn slice_to_array_ref<const START: usize, const END: usize, T, const N: usize>(
    slice: &[T],
) -> &[T; N] {
    const {
        assert!(
            START.checked_add(N).is_some(),
            "`START + N` would overflow a usize in slice_to_array_ref or slice_to_array",
        );
        assert!(
            START + N == END,
            "slice_to_array_ref or slice_to_array was called with incorrect START/END bounds",
        );
    }

    // The slice has the same length as the target array, so `try_into` succeeds
    #[expect(clippy::unwrap_used, reason = "we checked at compile time that this cannot fail")]
    slice[START..END].try_into().unwrap()
}

/// Consolidate where `.unwrap()` is called; in the context of mutexes, panicking when a mutex
/// is poisoned is usually the preferred behavior.
///
/// The implementation provided for `Mutex<T>` simply calls `.lock().unwrap()`.
pub trait LockOrPanic<T> {
    /// Consolidate where `.unwrap()` is called; in the context of mutexes, panicking when a mutex
    /// is poisoned is usually the preferred behavior.
    fn lock_or_panic(&self) -> MutexGuard<'_, T>;
}

impl<T> LockOrPanic<T> for Mutex<T> {
    /// Simply calls `.lock().unwrap()` on a `Mutex`, in order to consolidate where
    /// `.unwrap()` is called.
    /// # Panics
    /// Panics if the mutex is poisoned.
    #[inline]
    fn lock_or_panic(&self) -> MutexGuard<'_, T> {
        #[expect(
            clippy::unwrap_used,
            reason = "we want to panic if a mutex is poisoned",
        )]
        self.lock().unwrap()
    }
}

/// Map an enum into and from two other types using `From` and `TryFrom` (with unit error).
///
/// The enum type must be specified, followed by the type to map the enum into (`$into`),
/// optionally followed by the type to try to map into the enum (`$try_from`).
/// If `$try_from` is not specified, it is set to `$into`.
///
/// The two types must be similar enough that the same value expression (e.g., a numeric or string
/// literal) works for either; moreover, the expression must also be a valid pattern for a match
/// arm, so arbitrarily complicated expressions are not permitted.
/// In practice, the types should usually be the same, but specifying
/// two different types is useful for converting into `&'static str` and trying to convert
/// from `&str`, for example.
///
/// # Examples
///
/// ## Nonempty map, into and from the same type:
/// ```
/// # use prismarine_anchor_util::bijective_enum_map;
/// #[derive(Debug, PartialEq, Eq)]
/// enum AtMostTwo {
///     Zero,
///     One,
///     Two,
/// }
///
/// bijective_enum_map! {
///     AtMostTwo, u8,
///     Zero <=> 0,
///     One  <=> 1,
///     Two  <=> 2,
/// }
///
/// assert_eq!(u8::from(AtMostTwo::One), 1_u8);
/// assert_eq!(AtMostTwo::try_from(2_u8), Ok(AtMostTwo::Two));
/// assert_eq!(AtMostTwo::try_from(4_u8), Err(()));
/// ```
///
/// ## Empty map, into and from different types:
/// ```
/// # use prismarine_anchor_util::bijective_enum_map;
/// #[derive(Debug, PartialEq, Eq)]
/// enum Empty {}
///
/// // The trailing comma is always optional
/// bijective_enum_map! { Empty, &'static str, &str }
///
/// assert_eq!(Empty::try_from("42"), Err(()))
/// ```
///
/// ## Nonempty map, into and from the same type explicitly written twice:
/// ```
/// # use prismarine_anchor_util::bijective_enum_map;
/// #[derive(Debug, PartialEq, Eq)]
/// enum Enum {
///     One,
///     Two,
///     Three,
/// }
///
/// #[derive(Debug, PartialEq, Eq)]
/// enum Other {
///     Uno,
///     Dos,
///     Tres,
/// }
///
/// bijective_enum_map! {
///     Enum, Other, Other,
///     One   <=> Other::Uno,
///     Two   <=> Other::Dos,
///     Three <=> Other::Tres,
/// }
///
/// assert_eq!(Other::from(Enum::Three), Other::Tres);
/// // Note that this conversion cannot fail, but `bijective_enum_map` does not know that.
/// assert_eq!(Enum::try_from(Other::Uno), Ok(Enum::One));
/// ```
#[macro_export]
macro_rules! bijective_enum_map {
    { $enum_name:ty, $into:ty, $try_from:ty, $($body:tt)* } => {
        $crate::impl_from_enum! { $enum_name, $into, $($body)* }
        $crate::impl_enum_try_from! { $enum_name, $try_from, $($body)* }
    };

    { $enum_name:ty, $into:ty, $try_from:ty } => {
        $crate::impl_from_enum! { $enum_name, $into }
        $crate::impl_enum_try_from! { $enum_name, $try_from }
    };

    { $enum_name:ty, $both:ty, $($body:tt)* } => {
        $crate::impl_from_enum! { $enum_name, $both, $($body)* }
        $crate::impl_enum_try_from! { $enum_name, $both, $($body)* }
    };

    { $enum_name:ty, $both:ty } => {
        $crate::impl_from_enum! { $enum_name, $both }
        $crate::impl_enum_try_from! { $enum_name, $both }
    };
}

/// Helper macro for [`bijective_enum_map`] which provides a `From` implementation that
/// converts an enum into some type.
#[macro_export]
macro_rules! impl_from_enum {
    { $enum_name:ty, $into:ty, $($enum_variant:ident <=> $value:expr),+ $(,)? } => {
        impl ::core::convert::From<$enum_name> for $into {
            #[inline]
            fn from(value: $enum_name) -> Self {
                match value {
                    $( <$enum_name>::$enum_variant => $value ),+
                }
            }
        }
    };

    { $enum_name:ty, $into:ty $(,)? } => {
        impl ::core::convert::From<$enum_name> for $into {
            #[inline]
            fn from(value: $enum_name) -> Self {
                match value {}
            }
        }
    };
}

/// Helper macro for [`bijective_enum_map`] which provides a `TryFrom` implementation that
/// tries to convert some type into an enum.
///
/// Note that the `clippy::match_wildcard_for_single_variants`
/// and `non_exhaustive_omitted_patterns` lints
/// are not explicitly ignored here, so you can lint against them if you want to.
#[macro_export]
macro_rules! impl_enum_try_from {
    { $enum_name:ty, $try_from:ty, $($enum_variant:ident <=> $value:pat),+ $(,)? } => {
        impl ::core::convert::TryFrom<$try_from> for $enum_name {
            type Error = ();

            #[inline]
            fn try_from(value: $try_from) -> Result<Self, Self::Error> {
                #![allow(clippy::wildcard_enum_match_arm)]
                #![allow(unreachable_patterns)]
                Ok(match value {
                    $( $value => Self::$enum_variant ),+,
                    _ => return Err(()),
                })
            }
        }
    };

    { $enum_name:ty, $try_from:ty $(,)? } => {
        impl ::core::convert::TryFrom<$try_from> for $enum_name {
            type Error = ();

            #[inline]
            fn try_from(_value: $try_from) -> Result<Self, Self::Error> {
                Err(())
            }
        }
    };
}


#[cfg(test)]
mod tests {
    use super::bijective_enum_map;

    #[test]
    fn empty_both_specified() {
        #[derive(Debug, PartialEq, Eq)]
        enum Empty {}

        bijective_enum_map! {Empty, u8, u32}

        assert_eq!(Empty::try_from(2_u32), Err(()));
    }

    #[test]
    fn empty_one_specified() {
        #[derive(Debug, PartialEq, Eq)]
        enum Empty {}

        bijective_enum_map! {Empty, u8}

        assert_eq!(Empty::try_from(2_u8), Err(()));
    }

    #[test]
    fn nonempty_both_specified() {
        #[derive(Debug, PartialEq, Eq)]
        enum AtMostTwo {
            Zero,
            One,
            Two,
        }

        bijective_enum_map! {
            AtMostTwo, u8, u32,
            Zero <=> 0,
            One  <=> 1,
            Two  <=> 2,
        }

        assert_eq!(u8::from(AtMostTwo::One), 1_u8);
        assert_eq!(AtMostTwo::try_from(2_u32), Ok(AtMostTwo::Two));
        assert_eq!(AtMostTwo::try_from(4_u32), Err(()));
    }

    #[test]
    fn nonempty_one_specified() {
        #[derive(Debug, PartialEq, Eq)]
        enum AtMostTwo {
            Zero,
            One,
            Two,
        }

        bijective_enum_map! {
            AtMostTwo, u32,
            Zero <=> 0,
            One  <=> 1,
            Two  <=> 2,
        }

        assert_eq!(u32::from(AtMostTwo::One), 1_u32);
        assert_eq!(AtMostTwo::try_from(2_u32), Ok(AtMostTwo::Two));
        assert_eq!(AtMostTwo::try_from(4_u32), Err(()));
    }

    #[test]
    fn nonempty_to_enum_bijective() {
        #[derive(Debug, PartialEq, Eq)]
        enum Enum {
            One,
            Two,
            Three,
        }

        #[derive(Debug, PartialEq, Eq)]
        enum Other {
            Uno,
            Dos,
            Tres,
        }

        bijective_enum_map! {
            Enum, Other, Other,
            One   <=> Other::Uno,
            Two   <=> Other::Dos,
            Three <=> Other::Tres,
        }

        assert_eq!(Other::from(Enum::Three), Other::Tres);
        // Note that this conversion cannot fail, but `bijective_enum_map` does not know that.
        assert_eq!(Enum::try_from(Other::Uno), Ok(Enum::One));
    }

    #[test]
    fn nonempty_to_enum_injective() {
        #[derive(Debug, PartialEq, Eq)]
        enum Enum {
            One,
            Two,
            Three,
        }

        #[derive(Debug, PartialEq, Eq)]
        enum Other {
            Uno,
            Dos,
            Tres,
            Cuatro,
        }

        bijective_enum_map! {
            Enum, Other, Other,
            One   <=> Other::Uno,
            Two   <=> Other::Dos,
            Three <=> Other::Tres,
        }

        assert_eq!(Other::from(Enum::Three), Other::Tres);
        assert_eq!(Enum::try_from(Other::Uno), Ok(Enum::One));
        assert_eq!(Enum::try_from(Other::Cuatro), Err(()));
    }

    #[test]
    fn enum_to_string() {
        #[derive(Debug, PartialEq, Eq)]
        enum Empty {}

        #[derive(Debug, PartialEq, Eq)]
        enum Nonempty {
            Something,
        }

        bijective_enum_map! {Empty, &'static str, &str}
        bijective_enum_map! {
            Nonempty, &'static str, &str,
            Something <=> "Something",
        }

        assert_eq!(Empty::try_from("Anything"), Err(()));
        assert_eq!(Nonempty::try_from("Something"), Ok(Nonempty::Something));
        assert_eq!(Nonempty::try_from("Nothing"), Err(()));
    }

    #[test]
    fn trailing_commas() {
        enum Empty {}
        enum Nonempty {
            Something,
        }

        bijective_enum_map!(Empty, u8, u8);
        bijective_enum_map! { Empty, u16 };
        bijective_enum_map! {
            Empty, i8, i8,
        };
        bijective_enum_map! { Empty, i16, };

        bijective_enum_map!(Nonempty, u8, u8, Something <=> 0);
        bijective_enum_map! { Nonempty, u16, Something <=> 0};
        bijective_enum_map! {
            Nonempty, i8, i8, Something <=> 0,
        };
        bijective_enum_map! { Nonempty, i16, Something <=> 0,};
    }
}

#[cfg(doctest)]
pub mod compile_fail_tests {
    /// ```compile_fail,E0004
    /// use prismarine_anchor_util::bijective_enum_map;
    /// #[derive(Debug, PartialEq, Eq)]
    /// enum Nonempty {
    ///     Something,
    /// }
    ///
    /// bijective_enum_map! {Nonempty, u8}
    ///
    /// assert_eq!(Nonempty::try_from(2_u8), Err(()));
    /// ```
    pub fn _nonempty_but_nothing_provided() {}

    /// ```compile_fail,E0004
    /// use prismarine_anchor_util::bijective_enum_map;
    /// #[derive(Debug, PartialEq, Eq)]
    /// enum Nonempty {
    ///     Something,
    ///     SomethingElse,
    /// }
    ///
    /// bijective_enum_map! { Nonempty, u8, Something <=> 0 }
    ///
    /// assert_eq!(Nonempty::try_from(2_u8), Err(()));
    /// ```
    pub fn _nonempty_but_not_enough_provided() {}

    // Unfortunately, this compiles. Without #[deny(unreachable_patterns)],
    // there's simply a redundant pattern matching 0. And the macro ignores that lint
    // in case the wildcard arm is unreachable.
    // /// ```compile_fail
    // /// use prismarine_anchor_util::bijective_enum_map;
    // /// #[derive(Debug, PartialEq, Eq)]
    // /// enum AtMostTwo {
    // ///     Zero,
    // ///     One,
    // ///     Two,
    // /// }
    // ///
    // /// #[deny(unreachable_patterns)]
    // /// bijective_enum_map! {
    // ///     AtMostTwo, u8,
    // ///     Zero <=> 0,
    // ///     One  <=> 1,
    // ///     Two  <=> 0,
    // /// }
    // /// ```
    // pub fn _nonempty_not_injective() {}

    // Unfortunately, this compiles. Without #[deny(unreachable_patterns)],
    // there's simply a redundant pattern matching 0. And the macro ignores that lint
    // in case the wildcard arm is unreachable.
    // /// ```compile_fail
    // /// use prismarine_anchor_util::bijective_enum_map;
    // /// #[derive(Debug, PartialEq, Eq)]
    // /// enum AtMostTwo {
    // ///     Zero,
    // ///     One,
    // ///     Two,
    // /// }
    // ///
    // /// enum Other {
    // ///     Uno,
    // ///     Dos,
    // /// }
    // ///
    // /// bijective_enum_map! {
    // ///     AtMostTwo, Other,
    // ///     Zero <=> Other::Uno,
    // ///     One  <=> Other::Uno,
    // ///     Two  <=> Other::Dos,
    // /// }
    // ///
    // /// let _ = AtMostTwo::try_from(Other::Uno);
    // /// ```
    // pub fn _nonempty_to_enum_not_injective() {}

    // Surprisingly, this compiles. It defaults to `&'static str`, as far as I can tell.
    // /// ```compile_fail
    // /// use prismarine_anchor_util::bijective_enum_map;
    // /// enum Nonempty {
    // ///     Something,
    // /// }
    // ///
    // /// bijective_enum_map! {
    // ///     Nonempty, &str,
    // ///     Something <=> "Something",
    // /// }
    // ///
    // /// let _ = <&str>::from(Nonempty::Something);
    // /// ```
    // pub fn _enum_to_string_bad_lifetimes() {}

    // Doesn't seem to have a compiler error number
    /// ```compile_fail
    /// use prismarine_anchor_util::bijective_enum_map;
    /// enum Nonempty {
    ///     Something,
    /// }
    ///
    /// bijective_enum_map! {
    ///     Nonempty, u8
    ///     Something <=> 0
    /// }
    /// ```
    pub fn _missing_comma() {}

    /// ```compile_fail,E0080
    /// use prismarine_anchor_util::slice_to_array;
    /// let data = [0];
    ///
    /// let data_2 = slice_to_array::<{ usize::MAX }, 1, _, 2>(data.as_slice());
    /// ```
    pub fn _slice_to_array_overflow() {}

    /// ```compile_fail,E0080
    /// use prismarine_anchor_util::slice_to_array;
    /// let data = [0];
    ///
    /// let data_2 = slice_to_array::<0, 2, _, 1>(data.as_slice());
    /// ```
    pub fn _slice_to_array_invalid_bounds() {}

    /// ```compile_fail,E0080
    /// use prismarine_anchor_util::slice_to_array;
    /// let data = [0];
    ///
    /// let data_2 = slice_to_array::<1, 0, _, 1>(data.as_slice());
    /// ```
    pub fn _slice_to_array_end_after_start() {}
}
