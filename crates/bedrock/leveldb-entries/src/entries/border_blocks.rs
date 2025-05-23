#![allow(clippy::len_zero)]

use vecmap::VecSet;

use prismarine_anchor_mc_datatypes::ChunkColumn;

use crate::interface::{DataFidelity, ValueParseOptions, ValueToBytesOptions};


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorderBlocks(pub VecSet<ChunkColumn>);

impl BorderBlocks {
    pub fn parse(value: &[u8], opts: ValueParseOptions) -> Option<Self> {
        if value.len() < 1 {
            return None;
        }

        // Interestingly, if it's length zero, it simply doesn't get serialized (normally).
        let columns_len = if value[0] == 0 {
            256
        } else {
            usize::from(value[0])
        };

        if value.len() != 1 + columns_len {
            log::warn!(
                "Checksums with {} entries (according to header) was expected \
                 to have length {}, but had length {}",
                columns_len,
                1 + columns_len,
                value.len(),
            );
            return None;
        }

        let mut columns = value[1..].to_vec();

        if matches!(opts.data_fidelity, DataFidelity::Semantic) {
            columns.sort();
        }

        #[expect(
            clippy::unwrap_used,
            reason = "ChunkColumn::new succeeds iff both of its inputs are at most 15"
        )]
        let columns = columns
            .into_iter()
            .map(|pos| ChunkColumn::new(pos % 16, pos >> 4).unwrap())
            .collect();

        Some(Self(columns))
    }

    pub fn extend_serialized(&self, bytes: &mut Vec<u8>, opts: ValueToBytesOptions) {
        if self.0.is_empty() {
            return;
        }

        let mut columns = self.0
            .iter()
            .map(|column| {
                column.z() << 4 | column.x()
            })
            .collect::<Vec<u8>>();

        if matches!(opts.data_fidelity, DataFidelity::Semantic) {
            columns.sort();
        }

        // There are only 256 possible values of `ChunkColumn`, so the `VecSet` had nonzero
        // length which is at most 256. Casting this to u8 acts as it should (sends 256 to 0,
        // everything else is untouched).
        let len = columns.len() as u8;
        bytes.reserve(1 + columns.len());
        bytes.push(len);
        bytes.extend(columns);
    }

    pub fn to_bytes(&self, opts: ValueToBytesOptions) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes, opts);
        bytes
    }
}

// TODO: does attempting to serialize BorderBlocks data of length zero cause an error in MCBE?
