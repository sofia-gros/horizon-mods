# Optional Gravel-to-Flint crafting

The pinned TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` recipe
`common/flint` turns three Gravel into one Flint in a shapeless crafting grid.
It is available when `[recipes] addGravelToFlintRecipe = true` in
`config/tconstruct-common.toml`; an absent setting uses the source default,
true. False, an invalid supported value, unreadable config or malformed TOML
contributes no recipe.

The recipe is contributed to the native world's data files before its tables
are built, using Horizon's per-load `native_data_pack_files` boundary. Every
world open and `/reload` reads the setting again. It is not installed as an
unconditional resource recipe. The dedicated server uses its working directory
for config; single player uses the game root above `saves`. Remote clients do
not independently contribute a recipe from their own config.

The existing host overlay applies world datapacks above contributions and feeds
native crafting, recipe-book tables and LAN recipe serialization. An explicit
same-ID contribution is preserved. Datapacks can intentionally override the
recipe as usual. This only restores the recipe, not its separate recipe-unlock
advancement or the other missing config-gated vanilla alternatives.

Validation commands:

```
cargo test --locked --manifest-path tconstruct/tests/gravel-crafting/Cargo.toml
python3 -m unittest discover -s tconstruct/tests -p test_gravel_crafting.py -v
```

The Rust fixture executes the production contributor and config parser for
source-default/true, false/invalid/read-error, repeated true/false reloads,
separate game roots, and existing contributions. Python checks source quantities
and hook wiring. These checks do not prove actual crafting, in-game `/reload`,
recipe-book UI or LAN round trips; those still need gameplay validation.
