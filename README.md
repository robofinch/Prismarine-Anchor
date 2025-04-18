# Prismarine Anchor Editor
In-progress reimplementation of [Amulet Editor](https://www.amuletmc.com/) in Rust.
(Will likely end up very different, though, as perfectly matching Amulet is not a priority,
and code isn't copied from Amulet.)

### Licensing

Notable dependencies and inspirations include Amulet Editor, which is not open-source,
the awesome `quartz_nbt` (copying-and-pasting that is how the nbt crate here began),
and `rusty-leveldb` (their MCPE example and MemEnv struct were helpful, as well as their
main functionality as a LevelDB crate). Project Lodestone is a growing source for this project;
hopefully, some of the work here will also help Lodestone (whether as a dependency or
copying-and-pasting and adding a notice).

Much of the information for developing the NBT parser came from minecraft.wiki and wiki.bedrock.dev.

### Notes on conformance to copyright

This project has been cautious about copyright violations, and is well within the lines
as far as I am aware. (I'm not sure how many people bother to add the license or notice files
required by dependencies.)
Note that this project can rely on copyrighted information at runtime,
such as Mojang's publicly (and officially) available vanilla resource packs, and Amulet's
PyMCTranslate data in the Universal format. The parsers for Universal data of course have similar
functionality to PyMCTranslate out of necessity, but copyright protects expression, not any ideas;
and the Universal format isn't patented. Redistributing copyrighted data is still prohibited,
so the editor can download the data from its official distributor at compile time, but cannot
have that data compiled into it. (Well, the editor could be modified for personal use and have the
data compiled into it, but under Amulet's license, the editor then likely could't be shared with
others.)

Of course, avoiding copyright violations does not preclude unethical behavior;
I believe I've done my work ethically. I'm not sure what Amulet's reaction will be to projects like
this, so that's the one thing I'm worried about; however, with Amulet currently being free,
and with the utility and importance of open-source projects for the community, I'm comfortable with
my decision. I also put quite a lot of care into (to the greatest extent possible)
not looking at their source code, and reimplementing their functionality myself instead
of relying on their work. (With PyMCTranslate, some of their code had to be read to determine what
the Universal format even is, what with the years-out-of-date documentation.)
