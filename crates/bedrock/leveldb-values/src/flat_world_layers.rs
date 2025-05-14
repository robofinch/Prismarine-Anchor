use prismarine_anchor_util::InspectNone as _;


/// The block layers of a flat world, starting from the bottom, given as numerical block IDs.
///
/// For example, bedrock is ID `7`, dirt is `3`, and grass is `2`; the default flat world is then
/// `[7,3,3,2]`.
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone)]
pub struct FlatWorldLayers(pub Vec<u32>);

impl FlatWorldLayers {
    pub fn parse(value: &[u8]) -> Option<Self> {
        // The overall format is something like `b"[7,3,3,2]"`

        let value = value
            .strip_prefix(b"[")
            .inspect_none(|| log::warn!(
                "game_flatworldlayers lacked an opening '['",
            ))?
            .strip_suffix(b"]")
            .inspect_none(|| log::warn!(
                "game_flatworldlayers lacked a closing ']'",
            ))?;

        let layers = value
            .split(|&char_num| char_num == b',')
            .map(|num_slice| {
                let mut layer_num: u32 = 0;

                for &char_num in num_slice {
                    let digit = if char_num.is_ascii_digit() {
                        char_num - b'0'
                    } else {
                        return None;
                    };

                    layer_num = layer_num
                        .checked_mul(10)?
                        .checked_add(u32::from(digit))?;
                }

                Some(layer_num)
            })
            .collect::<Option<Vec<u32>>>()?;

        Some(Self(layers))
    }

    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        // We need at least `len` bytes for the numbers,
        // `len-1` or `0` for commas, and 2 for the endpoints
        if self.0.is_empty() {
            bytes.extend(b"[]");
        } else {
            bytes.reserve(1 + self.0.len() * 2);
            bytes.push(b'[');
            // In this branch, there is at least this first element
            bytes.extend(self.0[0].to_string().as_bytes());
            for layer in self.0.iter().skip(1) {
                bytes.extend(layer.to_string().as_bytes());
            }
            bytes.push(b']');
        }
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}
