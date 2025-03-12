use std::array;


pub fn allowed_unquoted(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+')
}

pub fn chars_to_u8(chars: [char; 2]) -> Option<u8> {
    let nibbles = [
        chars[0].to_digit(16)? as u8,
        chars[1].to_digit(16)? as u8,
    ];

    Some((nibbles[0] << 4) + nibbles[1])
}

pub fn chars_to_u16(chars: [char; 4]) -> Option<u16> {
    let nibbles = chars.map(|c| c.to_digit(16));

    let mut sum = nibbles[0]?;
    for i in 1..4 {
        sum = (sum << 4) +  nibbles[i]?;
    }

    Some(sum as u16)
}

pub fn chars_to_u32(chars: [char; 8]) -> Option<u32> {
    let nibbles = chars.map(|c| c.to_digit(16));

    let mut sum = nibbles[0]?;
    for i in 1..8 {
        sum = (sum << 4) +  nibbles[i]?;
    }

    Some(sum)
}

// Based on an answer from the Rust users forum
pub fn concat_arrays<T, const A: usize, const B: usize, const C: usize>(
    a: [T; A],
    b: [T; B],
) -> [T; C] {

    const {
        assert!(A + B == C, "incorrect output array length in call to `concat_arrays`");
    }

    let mut iter = a.into_iter().chain(b);
    array::from_fn(|_| iter.next().unwrap())
}
