# Tinkers' Construct for Horizon — Mixins migration

This is an early native source-mixin port of selected Tinkers' Construct
mechanics. It is not a complete port. It builds a separate Horizon executable;
it does not use the former fork's Wasm host API. The native code runs with the
user's privileges. The user's adoption decision is not a claim that current
Horizon documentation describes Mixins as its official exclusive mod system.

## What works today

In single-player worlds using the integrated server or in-frame mode, one flint and two sticks can be converted
into a flint pick head, wood binding and wood handle with `/tconstruct parts`.
`/tconstruct assemble` consumes those parts and produces a flint pickaxe.
The pickaxe mines stone, loses one durability per non-instant block, and its
item ID and damage survive a save and reload. At 85 damage it remains in the
inventory and cannot mine. Its current appearance uses the vanilla stone
pickaxe model, not a composed Tinkers texture. [Game screenshot](docs/game-smoke.png).

This is a narrow prototype: its command-based crafting is not the original
station UI, parts do not retain independent material data, and a broken tool
stops mining rather than mining slowly as in the original. Other game modes,
multiplayer, repair and traits are not verified.

## Requirements and setup

Linux, Python 3.12+, Git, Rust and Cargo, and your own prepared Horizon 26.3
resources are required. `compatibility.json` pins the source revisions. This
project does not distribute Minecraft game files or resources.

```sh
git clone https://github.com/horizon-rust/horizon.git ../horizon-official
git -C ../horizon-official checkout b6c936f82b08798057161bdfa577e3aac53fb91e
git clone https://github.com/horizon-rust/horizon-mixins.git ../horizon-mixins
git -C ../horizon-mixins checkout 0500ff24aa5933fca95a877ef41125353c4b4fe3
python3 tools/stage.py --horizon ../horizon-official --mixins ../horizon-mixins
cargo fetch --locked --manifest-path target/modded/Cargo.toml
cargo build --locked --offline --manifest-path target/modded/Cargo.toml \
  -p horizon-client --bin horizon-client --bin horizon-ctl --no-default-features --profile fast \
  --target-dir "$PWD/target/build"
python3 -m unittest discover -s tests -v
python3 tools/resources.py /path/to/prepared/26.3 target/resources
./target/build/fast/horizon-client --resources "$PWD/target/resources"
```

`stage.py` verifies both checkout HEADs and exports their pinned commits; local working-tree changes are not included. It exports a separate source tree,
and runs the Mixins CLI. It refuses an existing export; remove only your own
generated `target/original` when deliberately regenerating. Resource preparation
creates a separate overlay with links to the supplied base resources; keep the
base available. It refuses an existing destination. Generated trees and worlds
are not distributed. The prototype uses vanilla placeholder models, not the
original Tinkers artwork.

For the automated single-player game smoke test, run the following under a working
X11/Vulkan display (software Vulkan and Xvfb also work):

```sh
python3 tools/game_smoke.py --game target/build/fast/horizon-client \
  --ctl target/build/fast/horizon-ctl --resources target/resources
# Add --in-frame to test the alternate single-player path.
# Add --split-sticks to test one stick in each of two slots.
```

The script creates a fresh world under `target/`, checks consumption and
mining wear, saves, and writes a screenshot there. The independent save/reload
and broken-state checks are described in [validation](docs/validation.md).

See [coverage and roadmap](docs/roadmap.md) for scope and validation status.

## Provenance and recovery

The prior `main` was preserved at remote branch `backup/pre-mixins-92f696b`,
commit `92f696b475799403120330f6ad9c45c423de4ab7`. To inspect or restore it
without rewriting history, fetch that branch and create a new branch from it:

```sh
git fetch origin backup/pre-mixins-92f696b
git switch -c restore-pre-mixins FETCH_HEAD
```

Tinkers' Construct data is referenced at the revision in `compatibility.json`;
its MIT license and notice are included in [third_party/tconstruct/LICENSE](third_party/tconstruct/LICENSE).
The old environment's uncommitted chest work was not transferred to this checkout.
