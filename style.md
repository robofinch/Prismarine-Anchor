# Style conventions
- Use `make check` and `make clippy`, there should be no warnings.
- Any nonempty file should end with a newline, and no line may have trailing whitespace.
  (VSCode can enforce this.)
- Use `#[expect(..)]`, `todo!()`, `// TODO` comments as needed.
- Follow other conventions already in the code, like vertically aligning match arms
  and vertical function parameters.
- Most Rust naming conventions are followed, except for using `DB` instead of `Db`
  (the latter looks too weird, even if it's the pedantically correct `UpperCamelCase`).
- If using `rustfmt`, manually reformat everything afterwards
  (so, use `rustfmt` rarely if ever)
- The overall layout of a file is:
  - modules
  - imports
  - public imports (`pub use` or `pub(visibility) use`)
  - type aliases, followed by constants
  - the rest of the code
  - test module(s)
- In most of the code, there should be at most 1 line of whitespace
  between items, but between the above four categories, there should
  be 2 lines of whitespace.
- Items within each of the above categories,
  except the two imports categories,
  may be in any order that makes sense
  (in particular, all structs and then all impls is fine,
  and having each struct be directly followed by its impls
  is also fine.)
- When importing from a parent module, generally, only import items defined in that module;
  don't use the parent module's own imports, and instead reimport items with their full paths.
  The `prismarine-anchor-*-entries` crates are partial exceptions; they take advantage of parent
  modules' `pub use` statements.
- Imports (and public imports, respectively)
  are organized into the following groups:
  - `std` imports
  - imports from crates external to `prismarine-anchor`
  - imports from other crates in `prismarine-anchor`
  - `crate` imports, followed by `self` imports
- There should be no lines of whitespace within each of the above
  import groups, and one line of whitespace between different
  import groups.
- Within each import group, crates should be organized
  alphabetically (well, lexicographically since there are
  non-letter characters).
- The imports of a single crate should be organized into
  three categories:
  - modules (e.g., `std::io` or `std::slice`)
  - macros (e.g., `parse_macro_input`)
  - everything else (traits, types, constants, functions)
- Each import category of one crate should be organized into up to
  two `use` statements. Paths should be merged with `{}` where
  possible, and within a `{}`, should be ordered lexicographically.
  For instance, below `r` comes before `w`,
  even though `ZipError` would be after `SimpleFileOptions`.
  ```
  use zip::{result::ZipError, write::SimpleFileOptions};
  ```
  The first `use` statement of a category should have imports that
  use paths without any `{}` (except for a possible `{}` at the
  crate root), such as:
  ```
  use prismarine_anchor_nbt::{NbtCompound, settings::IoOptions};
  ```
  but not:
  ```
  use prismarine_anchor_nbt::io::{NbtIoError, write_compound};
  ```
  The second `use` statement of a category should have any imports
  with `{}` deeper in the path, such as the import just above, or
  ```
  use crate::{
      settings::{DepthLimit, SnbtParseOptions, SnbtVersion},
      tag::{NbtCompound, NbtList, NbtTag},
  };
  ```
- The import order rules are probably hard to understand just
  from reading them; look at some actual source code for more
  thorough examples.
