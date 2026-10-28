# Horizon Mixins mod development

This repository is the development foundation for a Tinkers’ Construct port to
[Horizon](https://github.com/horizon-rust/horizon) using
[Horizon Mixins](https://github.com/horizon-rust/horizon-mixins). It exports
pinned source commits and builds a separate native Horizon client. It does not
use the former fork's Wasm host API. Native Mixins code runs with the user's
privileges; this repository does not claim Wasm-equivalent isolation.

The adoption of Mixins as the project's chosen approach is a user direction,
not a claim that Horizon's public documentation mandates an exclusive mod
system.

## What works today

The independent fixture can be applied to the pinned official Horizon source,
compiled and run. Clicking a vanilla crafting table exercises its hook and
writes `independent fixture station-click hook executed` to the game log. This
is a development check, not Tinkers’ Construct gameplay.

The Part Builder, Tinker Station and tool work is an **unmerged preview** in
[draft PR #13](https://github.com/sofia-gros/horizon-mods/pull/13); its images
show that branch, not this foundation or current main. The original Mod is
not yet fully ported. See the [roadmap](docs/roadmap.md) for scope and the
[reference inventory](docs/upstream-inventory.json) for original source data.

## Requirements and setup

Use Linux, Git, Python 3.12+, Rust/Cargo, and your own prepared Horizon 26.3
resource directory. Horizon and Mixins are fixed in [compatibility.json](compatibility.json):

- Horizon `b6c936f82b08798057161bdfa577e3aac53fb91e`
- Horizon Mixins `0500ff24aa5933fca95a877ef41125353c4b4fe3`
- Tinkers’ Construct reference `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` (1.20.1)

```sh
git clone https://github.com/horizon-rust/horizon.git ../horizon-official
git -C ../horizon-official checkout --detach b6c936f82b08798057161bdfa577e3aac53fb91e
git clone https://github.com/horizon-rust/horizon-mixins.git ../horizon-mixins
git -C ../horizon-mixins checkout --detach 0500ff24aa5933fca95a877ef41125353c4b4fe3
python3 tools/stage.py --horizon ../horizon-official --mixins ../horizon-mixins
cargo fetch --locked --manifest-path target/modded/Cargo.toml
cargo build --locked --offline --manifest-path target/modded/Cargo.toml \
  -p horizon-client --bin horizon-client --bin horizon-ctl --no-default-features \
  --profile fast --target-dir "$PWD/target/build"
python3 -m unittest discover -s tests -v
./target/build/fast/horizon-client --resources /path/to/prepared/26.3
```

`stage.py` checks both checkout commits and exports those commits rather than
building dirty working-tree contents. By default it stages the independent
fixture; use `--mods /path/to/bundle-directory` for other Mixins bundles.
`--output-dir` selects a fresh output directory. Existing `original` or
`modded` directories are refused; the script does not delete them.

Under X11/Vulkan or Xvfb with software rendering, run the fixture in both game
modes against the same prepared resources:

```sh
python3 tests/game/fixture_smoke.py --game target/build/fast/horizon-client \
  --ctl target/build/fast/horizon-ctl --resources /path/to/prepared/26.3
python3 tests/game/fixture_smoke.py --game target/build/fast/horizon-client \
  --ctl target/build/fast/horizon-ctl --resources /path/to/prepared/26.3 --in-frame
```

See [validation](docs/validation.md) for tested versions and limits. CI checks
export, fixture transformation and compilation; it does not run the game.

## Reference and recovery

The earlier `main` was preserved at remote branch `backup/pre-mixins-92f696b`,
commit `92f696b475799403120330f6ad9c45c423de4ab7`. To inspect or restore it
without rewriting history:

```sh
git fetch origin backup/pre-mixins-92f696b
git switch -c restore-pre-mixins FETCH_HEAD
```

The uncommitted chest work in the older environment was not transferred into
this environment. It remains in that older environment and is not represented
by the backup branch.

The source reference is [SlimeKnights/TinkersConstruct](https://github.com/SlimeKnights/TinkersConstruct)
at the pinned commit. Its MIT license and attribution are included in
[third_party/tconstruct/LICENSE](third_party/tconstruct/LICENSE). The complete
source tree index and recipe catalog are reference inventories, not claims of
implemented features. The user's own prepared Horizon and game resources are
not distributed here.
