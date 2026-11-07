# Play on Linux

This manual build uses existing Horizon 26.3 resources and a separate game directory. The commands below have not been verified after the repository move. Earlier Linux gameplay checks used the previous layout.

## Prepare the game

Install Git, Python 3.12 or later, Rust, the native libraries required by Horizon, and a Vulkan-capable graphics driver. Prepare Horizon 26.3 resources with Horizon Launcher first. Clone this repository's `main` branch, then clone the Horizon and Horizon Mixins repositories and check out the revisions listed in `tconstruct/compatibility.json`.

From the repository root, replace the example paths with your own official checkout and resource paths. The source-stage and resource output directories must be fresh. The staging snippet first builds and validates both native packages from this checkout, then supplies their actual archive paths, expected SHA256 values and exact expanded members. It requires all full-plan guards before apply; do not continue after any refusal. See [development staging](development.md#prepare-a-modded-source-tree) for the archive contract and limits. No latest Linux build or gameplay success is claimed.

```sh
python3 - <<'PY_STAGE'
import os, subprocess, sys
from pathlib import Path
repo = Path.cwd()
horizon = Path('/path/to/horizon').resolve(strict=True)
mixins = Path('/path/to/horizon-mixins').resolve(strict=True)
output = repo / 'tconstruct/target/play'
sys.path.insert(0, str(repo / 'tconstruct/tools'))
import prepare_playtest_bundles as bundles
def run(argv, **kwargs):
    return subprocess.run(argv, check=True, **kwargs)
tc_mods, pluto, receipt = bundles.prepare(
    repo, repo / 'tconstruct/target/stage-package-inputs',
    run, sys.executable, repo, dict(os.environ))
bundles.verify_prepared(tc_mods, pluto, receipt)
run([sys.executable, repo / 'tconstruct/tools/stage.py',
     '--horizon', horizon, '--mixins', mixins, '--output-dir', output,
     '--mods', tc_mods, '--pluto-bundle', pluto,
     '--tconstruct-archive', tc_mods.parent / 'tconstruct.hnmod',
     '--pluto-archive', tc_mods.parent / 'plutolib.hnmod',
     '--tconstruct-archive-sha256', receipt['packages'][0]['archive_sha256'],
     '--pluto-archive-sha256', receipt['packages'][1]['archive_sha256']])
PY_STAGE
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
