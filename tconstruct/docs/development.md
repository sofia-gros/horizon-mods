# Build and test Tinkers' Construct

The mod applies its Mixins to the Horizon revision in `compatibility.json`. Builds normally use the latest official Horizon Mixins revision.

## Run the lightweight tests

From the repository root:

```sh
python3 -m unittest discover -s tests -v
python3 -m unittest discover -s tconstruct/tests -v
```

## Prepare a modded source tree

You need Python, Rust, Git, a checkout of the pinned Horizon revision, and a Horizon Mixins repository. From the repository root, replace the two paths with your local checkouts:

```sh
python3 tconstruct/tools/stage.py --horizon /path/to/horizon --mixins /path/to/horizon-mixins
```

The development target requires six matching PlutoLib bundles: `pluto_native_creative`, `pluto_transport_3a`, `pluto_tool_rules`, `pluto_component_display_3a`, `pluto_component_bar`, and `pluto_items_b2`. The Creative bundle provides the single `pluto-api` crate shared with the item adapter. Use the corresponding `--pluto-*` options shown by `stage.py --help` when the bundles are outside the repository checkout. Staging checks their IDs and Horizon revision and rejects the legacy adapters and duplicate bundles. Do not combine the old Creative or Item registries with the facade.

The facade integration targets PlutoLib commit `0f36b1fe779565fff37a445a485d920baa1bcf2f`. Install compatible PlutoLib native bundles alongside TConstruct as described in [PlutoLib native installation](../../pluto-lib/docs/native-installation.md); the launcher does not enforce the dependency. The combined TConstruct native installation and game behavior remain unverified.

The generated source is placed in `tconstruct/target/modded`. The resolved Mixins commit is printed and saved in `tconstruct/target/mixins-revision.txt`. To reproduce a build, pass `--mixins-revision FULL_COMMIT_SHA` and ensure that commit exists in the Mixins repository. Build the client with:

```sh
cargo check --locked --manifest-path tconstruct/target/modded/Cargo.toml -p horizon-client --bin horizon-client --no-default-features
```

Preparing game resources and running the game also require Horizon 26.3 resources. On Windows, the [playtest launcher](windows-playtest.md) handles this preparation for the playtest.

## Representative Anvil checks

The `tests/game/anvil_*.py` scripts exercise the real client using a dedicated test world under `tconstruct/target`. Supply `--game`, `--ctl`, `--resources`, and the same `--run-dir` inside that directory to each script. Run `anvil_acceptance.py` first to prepare the world, then `anvil_parts.py`, `anvil_compat.py`, `anvil_lan.py`, and `anvil_lan_upgrade.py`. These scripts place and break blocks in that test world; do not point them at a player world. Linux GUI tests need a display and Vulkan adapter. They do not replace Windows testing or a full survival run.

The branch manifest currently pins Horizon `cabe8b5de7d0b45541c923c4a6b3c5cb3e3686c7`. The isolated type check with eight omitted injections is described in [host upgrade notes](host-upgrade.md). Coordinate PlutoLib bundle revisions and compatibility pins before staging. The combined client has not been built or game-tested.
