Run `cargo run --package crawl-worlds --release -- path/to/minecraft/worlds/*` to crawl through
every world in a folder of Minecraft Bedrock worlds.

The code attempts to parse every single entry in each world, and round-trip the parsed value
back into bytes. Any problems that occur are printed, in addition to various other data.
By slightly modifying the code, you can inspect parts of your MCBE worlds.

The program has been run on around 20 gigabytes of Minecraft worlds; it works well.
If you encounter any issues, please reach out!
