use subslice_to_array::SubsliceToArray as _;

use crate::actor_id::ActorID;


#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone)]
pub struct ActorDigest(pub Vec<ActorID>);

impl ActorDigest {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() % 8 != 0 {
            log::warn!("ActorDigest data wasn't a multiple of 8");
            return None;
        }

        // We can process `value` in 8-byte chunks
        let actor_ids = value
            .chunks_exact(8)
            .map(|actor_id| {
                let actor_id = actor_id.subslice_to_array::<0, 8>();
                ActorID::parse(actor_id)
            })
            .collect();

        Some(Self(actor_ids))
    }

    #[inline]
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        bytes.reserve(self.0.len() * 8);
        for actor_id in &self.0 {
            bytes.extend(actor_id.to_bytes());
        }
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}
