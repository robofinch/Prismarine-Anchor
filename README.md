# Prismarine Anchor Editor
In-progress Minecraft world editor, inspired by [Amulet Editor](https://www.amuletmc.com/).
(Will likely end up very different, as perfectly matching Amulet is not a goal, and code here isn't
copied from Amulet.)

Correctness and thoroughness is a priority.

## Repository organization

* Crate names have `prismarine-anchor-` prefixes, but their folders elide that prefix.
* The crates in `crates/editor` are (or will be) related to the final application.
* The crates in `crates/foundation` do not rely on anything else in the Prismarine Anchor project,
  and may be useful for other people/projects.
* The crates in `crates/bedrock` are specific to Bedrock Edition.
* The crates the crates in `crates/cross-platform` are not specific to any version of Minecraft.
* The crates in the `crates/unstable` folder can be more or less ignored.

See `examples/crawl-worlds` for a binary that can crawl through every MCBE world whose directory
path it is passed.

## Sources

Notable dependencies, sources, and inspirations include:
* Amulet Editor, a current Minecraft world editor, which inspired this project.
* `quartz_nbt`, whose code was copied here as the starting point of the nbt crate here.
* `rusty-leveldb`, whose MCPE example and MemEnv struct were helpful, as well as their
  main functionality as a LevelDB crate.
* Project Lodestone, an ambitious project similar to this one, but with many contributors.
  Lodestone's documentation greatly aided in the development of the `leveldb-entries` crate.
  Hopefully, some of the work here will also help Lodestone (whether as a dependency or
  copying-and-pasting and adding a notice).
* minecraft.wiki and wiki.bedrock.dev provide large amounts of information, helpful for the NBT
  parser, `leveldb-entries`, and more.
* Rufus Atticus (and his `rbedrock` library) and LeviLamina's header files, which have very
  helpful for the `leveldb-entries` crate.
* Past me, who has provided so many old Bedrock Edition saves, which are invaluable for
  understanding Bedrock Edition save formats.

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
 * MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

So far, this codebase has been a personal project, and I think it would be fun to continue
working on this project by myself. If you're interesting in contributing to a project like this,
you should consider contributing to [Project Lodestone](https://github.com/Team-Lodestone).
I can be found on a variety of Discord servers (as ROBOTRON31415) if you'd like to discuss
this project.

If you encounter any problem with this project, though, please open an issue here.
