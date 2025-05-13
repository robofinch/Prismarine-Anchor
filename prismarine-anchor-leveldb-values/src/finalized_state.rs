use bijective_enum_map::injective_enum_map;
use subslice_to_array::SubsliceToArray as _;


// Based on rbedrock
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalizedState {
    NeedsInstaticking,
    NeedsPopulation,
    Done,
}

injective_enum_map! {
    FinalizedState, u32,
    NeedsInstaticking <=> 0,
    NeedsPopulation   <=> 1,
    Done              <=> 2,
}

impl FinalizedState {
    #[inline]
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() == 4 {
            let value = value.subslice_to_array::<0, 4>();
            Self::try_from(u32::from_le_bytes(value)).ok()
        } else {
            None
        }
    }

    #[inline]
    pub fn extend_serialized(self, bytes: &mut Vec<u8>) {
        bytes.extend(u32::from(self).to_le_bytes());
    }

    #[inline]
    pub fn to_bytes(self) -> Vec<u8> {
        u32::from(self).to_le_bytes().to_vec()
    }
}

