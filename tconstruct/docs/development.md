# Build and test Tinkers' Construct

Builds use the Horizon and Mixins revisions pinned in `compatibility.json`. Check upstream updates separately before changing the verified pair.

## Run the lightweight tests

From the repository root:

```sh
python3 -m unittest discover -s tests -v
python3 -m unittest discover -s tconstruct/tests -v
```

## Prepare a modded source tree

You need Python, Rust, Git, a checkout of the pinned Horizon revision, and a Horizon Mixins repository. From the repository root, replace the example paths with your local checkouts and installed bundle:

```sh
python3 tconstruct/tools/stage.py --horizon /path/to/horizon --mixins /path/to/horizon-mixins --pluto-bundle /path/to/installed.incoming/pluto_native_creative/mixins/pluto_native_creative
```

The development target requires the unified PlutoLib native bundle, built from PlutoLib `5520a221655e9ea5b55b9ce32a46bca511c3ae71`. The bundle contains Creative, item, Tool, transport, display and inventory APIs. It supplies one `pluto-api` facade and one `pluto-inventory-api` crate. Use the Mixins bundle directory extracted by the official launcher, or extract the `.hnmod` for staging with Horizon's native bundle reader:

```sh
python3 tconstruct/tools/extract_pluto.py --horizon /path/to/horizon --package /path/to/plutolib.hnmod --output /path/to/installed.incoming
```

The output directory must not already exist. Do not stage the old individual PlutoLib adapters alongside the unified bundle. Install PlutoLib separately from TConstruct; TConstruct still requires its own `.hnmod` and `.hmod`. See [PlutoLib native installation](../../pluto-lib/docs/native-installation.md). Combined installation and gameplay remain unverified.

The generated source is placed in `tconstruct/target/modded`. The resolved Mixins commit is printed and saved in `tconstruct/target/mixins-revision.txt`. The default is the pinned revision; `--mixins-revision FULL_COMMIT_SHA` selects an explicit revision for a separate compatibility test. Ensure the selected commit exists in the Mixins repository. Build the client with:

```sh
cargo check --locked --manifest-path tconstruct/target/modded/Cargo.toml -p horizon-client --bin horizon-client --no-default-features
```

Preparing game resources and running the game also require Horizon 26.3 resources. On Windows, the [playtest launcher](windows-playtest.md) handles this preparation for the playtest.

## Representative Anvil checks

The `tests/game/anvil_*.py` scripts exercise the real client using a dedicated test world under `tconstruct/target`. Supply `--game`, `--ctl`, `--resources`, and the same `--run-dir` inside that directory to each script. Run `anvil_acceptance.py` first to prepare the world, then `anvil_parts.py`, `anvil_compat.py`, `anvil_lan.py`, and `anvil_lan_upgrade.py`. These scripts place and break blocks in that test world; do not point them at a player world. Linux GUI tests need a display and Vulkan adapter. They do not replace Windows testing or a full survival run.

The development manifest targets Horizon `fd6cfda6bdef59a53aee830009f78c14832b0308` and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. The matching PlutoLib bundle and combined build are still being validated; see [compatibility](compatibility.md) for the previously tested pair.
