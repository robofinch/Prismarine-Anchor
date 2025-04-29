use prismarine_anchor_util::bijective_enum_map;
use prismarine_anchor_util::slice_to_array;


// Based on rbedrock
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalizedState {
    NeedsInstaticking,
    NeedsPopulation,
    Done,
}

bijective_enum_map! {
    FinalizedState, u32,
    NeedsInstaticking <=> 0,
    NeedsPopulation   <=> 1,
    Done              <=> 2,
}

impl FinalizedState {
    #[inline]
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() == 4 {
            let value = slice_to_array::<0, 4, _, 4>(value);
            Self::try_from(u32::from_le_bytes(value)).ok()
        } else {
            None
        }
    }

    #[inline]
    pub fn to_bytes(self) -> Vec<u8> {
        u32::from(self).to_le_bytes().to_vec()
    }
}

