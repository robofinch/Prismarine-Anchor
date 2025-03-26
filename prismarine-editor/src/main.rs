fn main() {
    println!("Hello, world!");
    println!("{}", std::mem::size_of::<prismarine_nbt::settings::SnbtParseOptions>());
    println!("{}", std::mem::size_of::<prismarine_nbt::settings::SnbtWriteOptions>());
    println!("{}", std::mem::size_of::<prismarine_nbt::settings::EnabledEscapeSequences>());
}
