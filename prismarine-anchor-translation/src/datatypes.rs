use std::{fmt, mem};
use std::borrow::Cow;
use std::{
    collections::{BTreeMap, VecDeque},
    fmt::{Display, Formatter},
};

use prismarine_anchor_nbt::{NbtCompound, NbtTag};


// A BTreeMap is used instead of HashMap in order to make caching of Blocks simpler
// (HashMaps do not implement Hash or Ord, BTreeMaps implement both).
pub type BlockProperties = BTreeMap<String, BlockProperty>;

pub const UNIVERSAL_NAMESPACE: &'static str = "universal_minecraft";
pub const MINECRAFT_NAMESPACE: &'static str = "minecraft";


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockPosition {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkPosition {
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingWorldPos {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// The possible variants of block properties in the Universal format
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlockProperty {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    String(String),
}

impl Display for BlockProperty {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Byte(n)   => write!(f, "{n}"),
            Self::Short(n)  => write!(f, "{n}"),
            Self::Int(n)    => write!(f, "{n}"),
            Self::Long(n)   => write!(f, "{n}"),
            Self::String(s) => write!(f, "{s}"),
        }
    }
}

impl From<BlockProperty> for NbtTag {
    fn from(value: BlockProperty) -> Self {
        match value {
            BlockProperty::Byte(n)   => Self::Byte(n),
            BlockProperty::Short(n)  => Self::Short(n),
            BlockProperty::Int(n)    => Self::Int(n),
            BlockProperty::Long(n)   => Self::Long(n),
            BlockProperty::String(s) => Self::String(s),
        }
    }
}

impl TryFrom<NbtTag> for BlockProperty {
    type Error = NbtTag;

    fn try_from(tag: NbtTag) -> Result<Self, Self::Error> {
        match tag {
            NbtTag::Byte(n)   => Ok(Self::Byte(n)),
            NbtTag::Short(n)  => Ok(Self::Short(n)),
            NbtTag::Int(n)    => Ok(Self::Int(n)),
            NbtTag::Long(n)   => Ok(Self::Long(n)),
            NbtTag::String(s) => Ok(Self::String(s)),
            _ => Err(tag),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Block {
    pub namespace: Cow<'static, str>,
    pub name: Cow<'static, str>,
    pub properties: BlockProperties,
    pub extra_layers: Vec<Block>,
}

impl Block {
    pub fn new(
        namespace: Cow<'static, str>,
        name: Cow<'static, str>,
        properties: Option<BlockProperties>,
        extra_layers: Option<Vec<Block>>,
    ) -> Self {
        Self {
            namespace,
            name,
            properties: properties.unwrap_or_else(|| BlockProperties::new()),
            // Vec::new() is cheap
            extra_layers: extra_layers.unwrap_or(Vec::new()),
        }
    }

    pub fn new_air() -> Self {
        Self {
            namespace: Cow::Borrowed(UNIVERSAL_NAMESPACE),
            name: Cow::Borrowed("air"),
            properties: BlockProperties::new(),
            extra_layers: Vec::new(),
        }
    }

    /// Recursively flattens the `extra_layers` of this block
    /// into a single vector (this block's own `extra_layers`), so that afterwards,
    /// the blocks in `extra_layers` have no extra layers of their own. The order of extra
    /// layers is preserved
    /// (a block's extra layers are after that block, and before its next sibling block).
    pub fn flatten_layers(&mut self) {
        let new_vec = Vec::with_capacity(self.extra_layers.len());
        // mem::replace returns the value of self.extra_layers *and* gives us ownership of it,
        // and sets self.extra_layers to new_vec. (that's why it can give us ownership)
        let extra_layers = mem::replace(&mut self.extra_layers, new_vec);

        let mut layer_queue = VecDeque::from(extra_layers);

        while let Some(mut block) = layer_queue.pop_front() {
            let inner_layers = mem::replace(&mut block.extra_layers, Vec::new());

            // There's no method like .extend_front(), we have to manually push each element in
            // the vec to the front of the queue (in the correct order)
            layer_queue.reserve(inner_layers.len());
            for block in inner_layers.into_iter().rev() {
                layer_queue.push_front(block);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockEntity {
    pub namespace: Cow<'static, str>,
    pub name: Cow<'static, str>,
    pub position: BlockPosition,
    pub nbt: NbtCompound,
    pub nbt_root_name: Cow<'static, str>,
}

impl BlockEntity {
    pub fn new(
        namespace: Cow<'static, str>,
        name: Cow<'static, str>,
        position: BlockPosition,
    ) -> Self {
        Self {
            namespace,
            name,
            position,
            nbt: NbtCompound::new(),
            nbt_root_name: Cow::Borrowed(""),
        }
    }

    pub fn new_with_nbt(
        namespace: Cow<'static, str>,
        name: Cow<'static, str>,
        position: BlockPosition,
        nbt: NbtCompound,
        nbt_root_name: Cow<'static, str>,
    ) -> Self {
        Self {
            namespace,
            name,
            position,
            nbt,
            nbt_root_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub namespace: Cow<'static, str>,
    pub name: Cow<'static, str>,
    pub position: FloatingWorldPos,
    pub nbt: NbtCompound,
    pub nbt_root_name: Cow<'static, str>,
}

impl Entity {
    pub fn new(
        namespace: Cow<'static, str>,
        name: Cow<'static, str>,
        position: FloatingWorldPos,
        nbt: NbtCompound,
        nbt_root_name: Cow<'static, str>,
    ) -> Self {
        Self {
            namespace,
            name,
            position,
            nbt,
            nbt_root_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockOrEntity {
    Block(Block, Option<BlockEntity>),
    Entity(Entity),
}

// I think I like the strategy of Item having an Option<Block> instead. might change this, then.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockItem {
    pub namespace: Cow<'static, str>,
    pub name: Cow<'static, str>,
    pub properties: BlockProperties,
    pub nbt: NbtCompound,
}

impl BlockItem {
    pub fn new(
        namespace: Cow<'static, str>,
        name: Cow<'static, str>,
        properties: BlockProperties,
        nbt: NbtCompound,
    ) -> Self {
        Self {
            namespace,
            name,
            properties,
            nbt,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub namespace: Cow<'static, str>,
    pub name: Cow<'static, str>,
    pub nbt: NbtCompound,
}

impl Item {
    pub fn new(
        namespace: Cow<'static, str>,
        name: Cow<'static, str>,
        nbt: NbtCompound,
    ) -> Self {
        Self {
            namespace,
            name,
            nbt,
        }
    }
}
