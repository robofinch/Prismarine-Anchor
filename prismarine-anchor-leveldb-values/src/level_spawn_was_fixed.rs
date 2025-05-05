#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelSpawnWasFixed(pub bool);

impl LevelSpawnWasFixed {
    #[inline]
    pub fn parse(value: &[u8]) -> Option<Self> {
        // I've only actually seen this be b"True", so I'm sort of just assuming
        // the alternative is b"False".
        if value == b"True" {
            Some(Self(true))
        } else if value == b"False" {
            Some(Self(false))
        } else {
            log::warn!("LevelSpawnWasFixed was not True or False; bytes: {value:?}");
            None
        }
    }

    #[inline]
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        if self.0 {
            bytes.extend(b"True");
        } else {
            bytes.extend(b"False");
        }
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}
