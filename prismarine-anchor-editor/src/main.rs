fn main() {
    println!("Hello, world!");
    println!("{}", std::mem::size_of::<prismarine_anchor_nbt::settings::SnbtParseOptions>());
    println!("{}", std::mem::size_of::<prismarine_anchor_nbt::settings::SnbtWriteOptions>());
    println!("{}", std::mem::size_of::<prismarine_anchor_nbt::settings::EnabledEscapeSequences>());
}
