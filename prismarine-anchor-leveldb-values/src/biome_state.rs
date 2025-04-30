#![allow(clippy::len_zero, reason = "clarity")]

use prismarine_anchor_util::bijective_enum_map;


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BiomeState {
    /// Note that this should be limited to `u8::MAX` (255) entries.
    OneByteBiomes(OneByteBiomeStates),
    /// Note that this should be limited to `u16::MAX` (65535) entries.
    TwoByteBiomes(TwoByteBiomeStates),
}

impl BiomeState {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() < 1 {
            return None;
        }

        let possible_num_entries = usize::from(value[0]);
        if value.len() == 1 + possible_num_entries * 2 {
            // This is the older `BiomeState` format, with a one-byte length header
            // followed by entries that are 2 bytes each (1-byte biomes, 1-byte values)
            let num_entries = possible_num_entries;

            let mut value = value.iter().copied().skip(1);
            let mut biome_snow_levels = Vec::with_capacity(num_entries);
            #[expect(
                clippy::unwrap_used,
                reason = "we know there are 2 bytes in value per entry",
            )]
            for _ in 0..num_entries {
                let biome = value.next().unwrap();
                let state = value.next().unwrap();
                biome_snow_levels.push((biome, state));
            }
            return OneByteBiomeStates::new(biome_snow_levels).map(Self::OneByteBiomes);
        }

        // Next, try the newer format, with a two-byte length header
        // followed by entries that are 3 bytes each (2-byte biomes, 1-byte values)
        if value.len() < 2 {
            return None;
        }

        let possible_num_entries = usize::from(u16::from_le_bytes([value[0], value[1]]));
        if value.len() == 2 + possible_num_entries * 3 {
            let num_entries = possible_num_entries;

            let mut value = value.iter().copied().skip(1);
            let mut biome_snow_levels = Vec::with_capacity(num_entries);
            #[expect(
                clippy::unwrap_used,
                reason = "we know there are 3 bytes in value per entry",
            )]
            for _ in 0..num_entries {
                let biome = u16::from_le_bytes([
                    value.next().unwrap(),
                    value.next().unwrap(),
                ]);
                let state = value.next().unwrap();
                biome_snow_levels.push((biome, state));
            }
            return TwoByteBiomeStates::new(biome_snow_levels).map(Self::TwoByteBiomes);
        }

        // At least for now, there are no other possibilities.
        None
    }

    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        match self {
            Self::OneByteBiomes(biome_states) => {
                let bytes_per_entry = 2;
                let num_entries = u8::try_from(biome_states.inner().len())
                    .unwrap_or(u8::MAX);
                bytes.reserve(1 + bytes_per_entry * usize::from(num_entries));

                bytes.push(num_entries);
                for (biome, state) in biome_states.entries().take(usize::from(num_entries)) {
                    bytes.push(biome);
                    bytes.push(state);
                }
            }
            Self::TwoByteBiomes(biome_states) => {
                let bytes_per_entry = 3;
                let num_entries = u16::try_from(biome_states.inner().len())
                    .unwrap_or(u16::MAX);
                bytes.reserve(2 + bytes_per_entry * usize::from(num_entries));

                bytes.extend(num_entries.to_le_bytes());
                for (biome, state) in biome_states.entries().take(usize::from(num_entries)) {
                    bytes.extend(biome.to_le_bytes());
                    bytes.push(state);
                }
            }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}

// These state values seem to have something to do with snow accumulation level.
// Maybe the maximum level that snow can accumulate to naturally.
// TODO: determine this
// Also, note that these could be backed by HashMaps,
// but the order of real game data is inconsistent, and have very, very few values.
// This makes things easier for testing to be able to round-trip,
// and is probably more performant, too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneByteBiomeStates(Vec<(u8, u8)>);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoByteBiomeStates(Vec<(u16, u8)>);

macro_rules! impl_n_byte_biome_states {
    ($name:ident, $biome_type:ty, $state_type:ty) => {
        impl $name {
            #[inline]
            pub fn new(biome_states: Vec<($biome_type, $state_type)>) -> Option<Self> {
                for (i, (biome, _)) in biome_states.iter().enumerate() {
                    for (other_biome, _) in biome_states.iter().skip(i + 1) {
                        if biome == other_biome {
                            return None;
                        }
                    }
                }

                Some(Self(biome_states))
            }

            #[inline]
            pub fn get_biome(&self, biome: $biome_type) -> Option<$state_type> {
                self.0.iter().find_map(|(key_biome, state)| {
                    if *key_biome == biome {
                        Some(*state)
                    } else {
                        None
                    }
                })
            }

            #[inline]
            pub fn set_biome(
                &mut self,
                biome: $biome_type,
                state: $state_type,
            ) -> Option<$state_type> {
                let old_entry = self
                    .0
                    .iter_mut()
                    .find(|(biome_key, _)| *biome_key == biome);

                if let Some((_, old_state)) = old_entry {
                    let old_state_copy = *old_state;
                    *old_state = state;
                    Some(old_state_copy)
                } else {
                    self.0.push((biome, state));
                    None
                }
            }

            #[inline]
            pub fn entries(&self) -> impl Iterator<Item = ($biome_type, $state_type)> {
                self.0.iter().copied()
            }

            #[inline]
            pub fn keys(&self) -> impl Iterator<Item = $biome_type> {
                self.0.iter().map(|(biome, _)| *biome)
            }

            #[inline]
            pub fn values(&self) -> impl Iterator<Item = $state_type> {
                self.0.iter().map(|(_, state)| *state)
            }

            #[inline]
            pub fn inner(&self) -> &Vec<($biome_type, $state_type)> {
                &self.0
            }

            #[inline]
            pub fn into_inner(self) -> Vec<($biome_type, $state_type)> {
                self.0
            }
        }
    };
}

impl_n_byte_biome_states!(OneByteBiomeStates, u8, u8);
impl_n_byte_biome_states!(TwoByteBiomeStates, u16, u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BytesPerBiome {
    One,
    Two,
}

bijective_enum_map! {
    BytesPerBiome, u8,
    One <=> 1,
    Two <=> 2,
}
