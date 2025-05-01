use indexmap::IndexMap;
use subslice_to_array::SubsliceToArray as _;


// Thanks to rbedrock, I didn't have to do as much work determining the binary format here
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checksums(pub IndexMap<ChecksumType, u64>);

impl Checksums {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() < 4 {
            return None;
        }

        let num_entries = u32::from_le_bytes(value.subslice_to_array::<0, 4>());
        let num_entries = usize::try_from(num_entries).ok()?;

        if value.len() != 4 + num_entries * 11 {
            return None;
        }

        // We can process value in chunks of 11 bytes
        let mut value = &value[4..];
        let mut checksums = IndexMap::with_capacity(num_entries);
        for _ in 0..num_entries {
            let tag           = value.subslice_to_array::<0, 2>();
            let subtag        = value[2] as i8;
            let checksum_hash = value.subslice_to_array::<3, 11>();
            value = &value[11..];

            checksums.insert(
                ChecksumType::parse(u16::from_le_bytes(tag), subtag)?,
                u64::from_le_bytes(checksum_hash),
            );
        }

        Some(Self(checksums))
    }

    pub fn extend_serialized(&self, _bytes: &mut Vec<u8>) {
        todo!()
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChecksumType {
    Data2D,
    SubchunkBlocks(i8),
    BlockEntities,
    Entities,
}

impl ChecksumType {
    #[inline]
    pub fn parse(tag: u16, y_index: i8) -> Option<Self> {
        match (tag, y_index) {
            (45, 0)       => Some(Self::Data2D),
            (47, y_index) => Some(Self::SubchunkBlocks(y_index)),
            (49, 0)       => Some(Self::BlockEntities),
            (50, 0)       => Some(Self::Entities),
            _             => None,
        }
    }

    #[inline]
    pub fn to_tag_and_subtag(self) -> (u16, i8) {
        match self {
            Self::Data2D                  => (45, 0),
            Self::SubchunkBlocks(y_index) => (47, y_index),
            Self::BlockEntities           => (49, 0),
            Self::Entities                => (50, 0),
        }
    }
}
