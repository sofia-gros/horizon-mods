# Play on Linux

This manual build uses existing Horizon 26.3 resources and a separate game directory. The commands below have not been verified after the repository move. Earlier Linux gameplay checks used the previous layout.

## Prepare the game

Install Git, Python 3.12 or later, Rust, the native libraries required by Horizon, and a Vulkan-capable graphics driver. Prepare Horizon 26.3 resources with Horizon Launcher first. Clone this repository's `main` branch, then clone the Horizon and Horizon Mixins repositories and check out the revisions listed in `tconstruct/compatibility.json`.

From the repository root, replace the three example paths with your own checkout and resource paths. The output directories must not already exist.

```sh
python3 tconstruct/tools/stage.py --horizon /path/to/horizon --mixins /path/to/horizon-mixins --output-dir tconstruct/target/play
cargo build --locked --manifest-path tconstruct/target/play/modded/Cargo.toml -p horizon-client --bin horizon-client --no-default-features --profile fast --target-dir tconstruct/target/build
python3 tconstruct/tools/resources.py /path/to/horizon-resources tconstruct/target/play/item-resources
tconstruct/target/build/fast/horizon-client --tconstruct-prepare tconstruct/target/play/item-resources tconstruct/content tconstruct/target/play/resources
python3 tconstruct/tools/prepare_horizon_source_data.py /path/to/horizon-resources tconstruct/target/play/resources/minecraft-original-pack
python3 tconstruct/tools/prepare_bundled_forge.py tconstruct/target/play/resources/forge-original-pack
```

## Start a separate world

```sh
mkdir -p "$HOME/TConstruct-Horizon-Playtest"
HORIZON_HOME="$HOME/TConstruct-Horizon-Playtest" tconstruct/target/build/fast/horizon-client --resources "$PWD/tconstruct/target/play/resources" --world TConstruct-Playtest --seed 42 --creative
```

This build omits audio. See [Making parts and tools](playing.md) for examples. Windows users can follow the [Windows guide](windows-playtest.md).
