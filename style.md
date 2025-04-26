# Style conventions
- Use `make check` and `make clippy`, there should be no warnings.
- All lines should have a newline at the end (i.e., the last one),
  and no line may have trailing whitespace.
  (VSCode can enforce this.)
- Use `#[expect(..)]`, `todo!()`, `// TODO` comments as needed.
- If using `rustfmt`, manually reformat everything afterwards
  (so, use `rustfmt` rarely if ever)
- The overall layout of a file is:
  - modules
  - imports
  - public imports (`pub use` or `pub(visibility) use`)
  - type aliases, followed by constants
  - the rest of the code
- In most of the code, there should be at most 1 line of whitespace
  between items, but between the above four categories, there should
  be 2 lines of whitespace.
- Items within each of the above categories,
  except the two imports categories,
  may be in any order that makes sense
  (in particular, all structs and then all impls is fine,
  and having each struct be directly followed by its impls
  is also fine.)
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
