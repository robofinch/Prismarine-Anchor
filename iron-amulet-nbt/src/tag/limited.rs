use std::fmt::{Display, Debug, Formatter, Result};

use crate::tag::{DepthLimit, NbtCompound, NbtList, NbtTag};

macro_rules! depth_limited {
    ($tag:ty, $name: ident) => {
        pub struct $name<'a> {
            tag: &'a $tag,
            depth_limit: DepthLimit
        }

        impl<'a> $name<'a> {
            pub fn new(tag: &'a $tag, depth_limit: DepthLimit) -> Self {
                Self {
                    tag,
                    depth_limit
                }
            }
        }

        impl<'a> Display for $name<'a> {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                self.tag.to_formatted_snbt(f, self.depth_limit)
            }
        }

        impl<'a> Debug for $name<'a> {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                self.tag.to_formatted_snbt(f, self.depth_limit)
            }
        }
    };
}

depth_limited!(NbtTag, TagWithLimit);
depth_limited!(NbtList, ListWithLimit);
depth_limited!(NbtCompound, CompoundWithLimit);
