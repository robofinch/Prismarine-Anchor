all:	clippy check

clippy:
	cargo clippy --no-default-features
	cargo clippy
	cargo clippy --all-features

# NBT features:
# preserve_order      = [ "dep:indexmap" ]
# comparable          = []
# float_cmp           = [ "comparable", "dep:float-cmp" ]
# named_escapes       = [ "dep:unicode_names2" ]
# allow_list_root     = []
# configurable_depth  = []
# serde               = [ "dep:serde" ]
# derive_serde        = [ "serde/serde_derive"]
# derive_standard     = []
# allow_any_root      = []
# allow_unnamed_root  = [ "allow_any_root" ]
# Technically also the default feature.

# Combinations to check:
# power set of preserve_order, comparable, float_cmp, serde, allow_unnamed_root
# plus check each feature, with a depth of 2 just in case

# NOTE for VSCode users:
# The first of the four checks performed below makes for a good check for rust-analyzer.
# Example .vscode/settings.json:
# {
#     "rust-analyzer.check.overrideCommand": [
#         "cargo",
#         "hack",
#         "check",
#         "--message-format=json",
#         "--feature-powerset",
#         "--exclude",
#         "prismarine-anchor-leveldb-values",
#         "--exclude",
#         "prismarine-anchor-nbt",
#     ],
#     "rust-analyzer.checkOnSave": true,
#     // "rust-analyzer.cargo.features": "all"
# }
check:
	cargo hack check --feature-powerset --exclude prismarine-anchor-nbt --exclude prismarine-anchor-leveldb-values
	cargo hack check --each-feature --package prismarine-anchor-leveldb-values
	cargo hack check --feature-powerset --package prismarine-anchor-nbt --depth 2
	cargo hack check --feature-powerset --package prismarine-anchor-nbt --exclude-features named_escapes,allow_list_root,configurable_depth,derive_serde,derive_standard,allow_any_root,default
