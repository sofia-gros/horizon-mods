# Optional Gravel-to-Flint crafting

The pinned TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` recipe
`common/flint` turns three Gravel into one Flint in a shapeless crafting grid.
It is available when `[recipes] addGravelToFlintRecipe = true` in
`config/tconstruct-common.toml`; an absent setting uses the source default,
true. False, an invalid supported value, unreadable config or malformed TOML
contributes neither recipe nor discovery advancement.

The recipe is contributed to the native world's data files before its tables
are built, using Horizon's per-load `native_data_pack_files` boundary. Every
world open and `/reload` reads the setting again. It is not installed as an
unconditional resource recipe. The dedicated server uses its working directory
for config; single player uses the game root above `saves`. Remote clients do
not independently contribute either file from their own config.

The existing host overlay applies world datapacks above contributions and feeds
native crafting, recipe-book tables and LAN recipe serialization. An explicit
same-ID recipe contribution is preserved and suppresses this helper's own
advancement, since the replacement is owned by the other contributor. An
existing same-ID advancement is also preserved. World datapacks can
intentionally override either file as usual.

The source recipe-book advancement is emitted alongside the recipe under
`advancement/tconstruct:recipes/misc/common/flint`. Gravel possession OR the
recipe-unlocked event discovers `tconstruct:common/flint`. The original hidden
parent, direct-item criterion, requirements and reward are preserved; only the
host's string-valued `recipes` event predicate is adapted. No unconditional
advancement is installed, and no progress is forcibly revoked when config is
disabled. This does not implement other config-gated vanilla alternatives.

Exact pinned source advancement bytes and hash are retained in
`tests/fixtures/flint_discovery/`.

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

The discovery addition received brief static review only. Contributor and
source regression coverage was extended but not run; no compilation, CI
inspection, version checks or merge was performed. Actual load/reload removal,
recipe-book state, progress persistence and LAN remain unverified.
