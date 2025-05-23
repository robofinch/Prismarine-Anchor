all:	clippy test check check_web

clippy:
	cargo clippy --no-default-features
	cargo clippy
	cargo clippy --all-features

test:
	cargo test

# NBT features:
# default             = [ "named_escapes" ]
# preserve_order      = [ "dep:indexmap" ]
# float_cmp           = [ "dep:float-cmp" ]
# named_escapes       = [ "dep:unicode_names2" ]
# allow_any_root      = []
# configurable_depth  = []
# serde               = [ "dep:serde" ]
# derive_serde        = [ "serde/serde_derive"]
# derive_standard     = []
# js                  = [ "getrandom/js" ]

# Combinations to check:
# power set of preserve_order, float_cmp, serde, allow_any_root
# plus check each feature, with a depth of 2 just in case

check:
	cargo hack check --feature-powerset --exclude prismarine-anchor-nbt --exclude prismarine-anchor-mc-datatypes
	cargo hack check --each-feature --package prismarine-anchor-mc-datatypes
	cargo hack check --feature-powerset --package prismarine-anchor-nbt --depth 2
	cargo hack check --feature-powerset --package prismarine-anchor-nbt --exclude-features named_escapes,configurable_depth,derive_serde,derive_standard,default

check_web:
	cargo hack check --target wasm32-unknown-unknown --feature-powerset --package prismarine-anchor-world --features js
	cargo hack check --target wasm32-unknown-unknown --feature-powerset --package prismarine-anchor-nbt --depth 2 --features js
	cargo hack check --target wasm32-unknown-unknown --feature-powerset --package prismarine-anchor-nbt --exclude-features named_escapes,configurable_depth,derive_serde,derive_standard,default --features js
	cargo hack check --target wasm32-unknown-unknown --each-feature --package prismarine-anchor-mc-datatypes
	cargo hack check --target wasm32-unknown-unknown --feature-powerset --exclude prismarine-anchor-nbt --exclude prismarine-anchor-world

possible_missing_commas:
	rg -U '[^,]\n[ ]*\]'
	rg -U '[^,]\n[ ]*\)'
