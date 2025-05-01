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
