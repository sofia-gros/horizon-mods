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

The development branch also requires PlutoLib’s `pluto_native_creative` and experimental `pluto_transport_3a` bundles. `stage.py` looks for it under `pluto-lib/mixins/pluto_native_creative`; use `--pluto-native-creative /path/to/pluto_native_creative` while that bundle is maintained separately. Supply the experimental transport bundle with `--pluto-transport /path/to/pluto_transport_3a`; this draft uses PlutoLib commit `b0e7fa84c736e2fb49a4dba48270580d59861e2c`, and the transport bundle is not yet in the distributed package. It rejects the legacy PlutoLib paging adapter because the target Horizon supplies paging. The intended launcher installation uses separate enabled TConstruct and PlutoLib native bundles built together, plus the TConstruct content mod. Install one matching PlutoLib native Creative bundle; the launcher manifest does not enforce the dependency. The bundle is included in the repository; see [PlutoLib native installation](../../pluto-lib/docs/native-installation.md) for packaging and installation. The combined native path has not passed a full build or game test.

The generated source is placed in `tconstruct/target/modded`. The resolved Mixins commit is printed and saved in `tconstruct/target/mixins-revision.txt`. To reproduce a build, pass `--mixins-revision FULL_COMMIT_SHA` and ensure that commit exists in the Mixins repository. Build the client with:

```sh
cargo check --locked --manifest-path tconstruct/target/modded/Cargo.toml -p horizon-client --bin horizon-client --no-default-features
```

Preparing game resources and running the game also require Horizon 26.3 resources. On Windows, the [playtest launcher](windows-playtest.md) handles this preparation for the playtest.

## Representative Anvil checks

The `tests/game/anvil_*.py` scripts exercise the real client using a dedicated test world under `tconstruct/target`. Supply `--game`, `--ctl`, `--resources`, and the same `--run-dir` inside that directory to each script. Run `anvil_acceptance.py` first to prepare the world, then `anvil_parts.py`, `anvil_compat.py`, `anvil_lan.py`, and `anvil_lan_upgrade.py`. These scripts place and break blocks in that test world; do not point them at a player world. Linux GUI tests need a display and Vulkan adapter. They do not replace Windows testing or a full survival run.
