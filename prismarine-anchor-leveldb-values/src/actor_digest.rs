use subslice_to_array::SubsliceToArray as _;

use crate::actor_id::ActorID;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorDigest(pub Vec<ActorID>);

impl ActorDigest {
    pub fn parse(value: &[u8]) -> Option<Self> {
        if value.len() % 8 != 0 {
            return None;
        }

        let mut actor_ids = Vec::with_capacity(value.len() / 8);

        // We can process `value` in 8-byte chunks
        let mut value = value;
        while !value.is_empty() {
            let next_actor_id = value.subslice_to_array::<0, 8>();
            value = &value[8..];

            actor_ids.push(ActorID::parse(next_actor_id));
        }

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
