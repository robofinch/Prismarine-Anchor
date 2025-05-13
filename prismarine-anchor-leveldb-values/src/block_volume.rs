use std::num::NonZeroU32;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockVolume {
    pub low_x:   i32,
    pub low_y:   i32,
    pub low_z:   i32,
    pub width_x: NonZeroU32,
    pub width_y: NonZeroU32,
    pub width_z: NonZeroU32,
}

impl BlockVolume {
    pub fn parse(value: [u8; 24]) -> Option<Self> {
        let low_x  = i32::from_le_bytes([value[0],  value[ 1], value[ 2], value[ 3]]);
        let low_y  = i32::from_le_bytes([value[4],  value[ 5], value[ 6], value[ 7]]);
        let low_z  = i32::from_le_bytes([value[8],  value[ 9], value[10], value[11]]);
        let high_x = i32::from_le_bytes([value[12], value[13], value[14], value[15]]);
        let high_y = i32::from_le_bytes([value[16], value[17], value[18], value[19]]);
        let high_z = i32::from_le_bytes([value[20], value[21], value[22], value[23]]);

        if low_x <= high_x && low_y <= high_y && low_z <= high_z {
            #[expect(
                clippy::unwrap_used,
                reason = "`.saturating_add(1)` ensures that the values are nonzero",
            )]
            Some(Self {
                low_x,
                low_y,
                low_z,
                width_x: NonZeroU32::new(high_x.abs_diff(low_x).saturating_add(1)).unwrap(),
                width_y: NonZeroU32::new(high_y.abs_diff(low_y).saturating_add(1)).unwrap(),
                width_z: NonZeroU32::new(high_z.abs_diff(low_z).saturating_add(1)).unwrap(),
            })
        } else {
            #[expect(clippy::uninlined_format_args, reason = "line length")]
            {
                log::warn!(
                    "Invalid BlockVolume; x: ({} ..= {}), y: ({} ..= {}), z: ({} ..= {})",
                    low_x, high_x, low_y, high_y, low_z, high_z,
                );
            };
            None
        }
    }

    #[inline]
    pub fn extend_serialized(&self, bytes: &mut Vec<u8>) {
        let high_x = self.low_x.saturating_add_unsigned(self.width_x.get() - 1);
        let high_y = self.low_y.saturating_add_unsigned(self.width_y.get() - 1);
        let high_z = self.low_z.saturating_add_unsigned(self.width_z.get() - 1);

        bytes.extend(self.low_x.to_le_bytes());
        bytes.extend(self.low_y.to_le_bytes());
        bytes.extend(self.low_z.to_le_bytes());
        bytes.extend(high_x.to_le_bytes());
        bytes.extend(high_y.to_le_bytes());
        bytes.extend(high_z.to_le_bytes());
    }

    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.extend_serialized(&mut bytes);
        bytes
    }
}
