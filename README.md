# Prismarine Editor
In-progress reimplementation of [Amulet Editor](https://www.amuletmc.com/) in Rust.
(Might end up very different, though, as perfectly matching Amulet is not a priority,
and this is a clean-room-style reimplementation not based on Amulet's source.)

### Licensing

Licensing details are in progress. Do not interpret any of the following as providing you
with any license to use or copy this version of the project. Licenses as roughly described
below will likely be provided at some point in the future, but are not provided now.

First, note that the below is me being pedantic and thorough and handling edge cases.
Don't let it concern you very much, this is a "just in case" sort of thing.


Most of this project will likely not be open source,
and will prohibit competing with Amulet similarly to the
[Amulet Team License 1.0.0](https://github.com/Amulet-Team/Amulet-NBT/blob/4.0/LICENSE),
with only the NBT crate (prismarine-nbt) open-sourced.

The licensing of previous versions of this repo may or may not be legally accurate, and if
you are viewing the history of this repo, you should not rely on everything actually being
open-source in previous versions.

Notable dependencies or inspirations include Amulet Editor, which is not open-source,
the awesome `quartz_nbt` (open source), and `rusty-leveldb`'s MCPE example (open source).
Much of the information for developing the NBT parser came from minecraft.wiki and wiki.bedrock.dev.

The intent will be for this project to remain legally permissible even if it counts as a competitor
of Amulet Editor, while no user or derivative of this project may compete with Amulet Editor,
just as no user or derivative of Amulet Editor may compete with it. (Note that "derivative" is
used in the more legal, copyright-context sense, not general colloquial usage.)

This project does not intend to harm Amulet Editor, or steal Amulet Editor's users;
ensuring that this project does not include code or data copied or modified from Amulet Editor
(thereby ensuring the noncompete of Amulet's copyright does not apply to this project)
is not intended to be malicious or unethical,
but to doubly-ensure this project does not violate any laws or copyrights.

This intent cannot guarantee what actually happens, but I (the current developer of this project)
want to make it clear that I like Amulet, and if this project does end up harming Amulet,
I will want to do something about it (but don't want to be legally liable if that happens).
Count this as me being pedantic about copyrights to avoid relying solely on a looser sense
of trust and goodwill, but I do hope that trust and goodwill is still present.

Again, even if this project does not include data or code copied or modified from Amulet codebases,
and successfully avoids running afoul of the noncompete, the noncompete will still apply
to users or derivatives of this project.
