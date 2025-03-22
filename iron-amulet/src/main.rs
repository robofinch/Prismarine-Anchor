fn main() {
    println!("Hello, world!");
    println!("{}", std::mem::size_of::<iron_amulet_nbt::settings::SnbtParseOptions>());
    println!("{}", std::mem::size_of::<iron_amulet_nbt::settings::SnbtWriteOptions>());
    println!("{}", std::mem::size_of::<iron_amulet_nbt::settings::EnabledEscapeSequences>());
}
