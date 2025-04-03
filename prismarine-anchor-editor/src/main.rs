use std::mem::size_of;
use prismarine_anchor_nbt::{settings, snbt};

fn main() {
    println!("Hello, world!");
    println!("{}", size_of::<settings::SnbtParseOptions>());
    println!("{}", size_of::<settings::SnbtWriteOptions>());
    println!("{}", size_of::<settings::EnabledEscapeSequences>());
    println!("{}", size_of::<snbt::VerifiedSnbt>());
    println!("{}", size_of::<Option<snbt::VerifiedSnbt>>());
}
