use crate::bijective_enum_map;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorDigestVersion {
    V1_18_30,
}

bijective_enum_map! {
    ActorDigestVersion, u8, u8,
    V1_18_30 <=> 0,
}
