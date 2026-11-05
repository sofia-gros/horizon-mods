#!/usr/bin/env bash
# Compile the bounded source fixture against a compatible local Horizon host.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
: "${HORIZON_HOST:?Set HORIZON_HOST to a local Horizon 0c7b5b61 checkout or staged workspace}"
HORIZON_HOST=$(cd "$HORIZON_HOST" && pwd)
EXPECTED=0c7b5b61a5e746d621afffba919b12aeccf9c0fa
ACTUAL=$(git -C "$HORIZON_HOST" rev-parse HEAD 2>/dev/null) || {
    echo 'HORIZON_HOST must be a verifiable Git checkout of the pinned host.' >&2; exit 1;
}
if [[ "$ACTUAL" != "$EXPECTED" ]] || [[ -n "$(git -C "$HORIZON_HOST" status --porcelain --untracked-files=no)" ]]; then
    echo "HORIZON_HOST must have a clean tracked tree at $EXPECTED." >&2; exit 1
fi
export ROOT HORIZON_HOST
WORK=$(mktemp -d "${TMPDIR:-/tmp}/melter-fuel-runtime.XXXXXXXX")
trap 'rm -rf "$WORK"' EXIT
export WORK
export MELTER_RUNTIME_EXTRACT="$WORK/extracted"
python3 - <<'PY'
import json, os
from pathlib import Path
host = Path(os.environ['HORIZON_HOST'])
root = Path(os.environ['ROOT'])
work = Path(os.environ['WORK'])
for crate in ('horizon-world', 'horizon-items'):
    if not (host / 'crates' / crate / 'Cargo.toml').is_file():
        raise SystemExit(f'missing host crate: {crate}')
def quoted(path):
    return json.dumps(str(path))
manifest = f'''[package]
name = "melter-fuel-runtime-fixture"
version = "0.1.0"
edition = "2024"
[dependencies]
horizon-world = {{ path = {quoted(host / 'crates/horizon-world')} }}
horizon-items = {{ path = {quoted(host / 'crates/horizon-items')} }}
serde_json = "1"
[lib]
path = {quoted(root / 'tconstruct/tests/rust/melter_fuel_runtime.rs')}
'''
(work / 'Cargo.toml').write_text(manifest)
PY
export SURVIVAL_RECIPES="$WORK/recipes"
python3 - <<'PYRECIPES'
import os, zipfile
from pathlib import Path
root = Path(os.environ['ROOT'])
out = Path(os.environ['SURVIVAL_RECIPES'])
out.mkdir()
with zipfile.ZipFile(root / 'tconstruct/content/reference/tconstruct-recipes.zip') as archive:
    for path in archive.namelist():
        if path.startswith('data/tconstruct/recipes/') and path.endswith('.json'):
            # Flat names prevent archive paths from becoming filesystem paths.
            (out / (path.removeprefix('data/tconstruct/recipes/').replace('/', '__'))).write_bytes(archive.read(path))
PYRECIPES
python3 - <<'PYEXTRACT'
import os
from pathlib import Path
root=Path(os.environ['ROOT'])/'tconstruct/mixins/tconstruct'
out=Path(os.environ['MELTER_RUNTIME_EXTRACT']); out.mkdir()
text=(root/'source-machine-runtime.rs').read_text()
def section(text,start,end):
    if text.count(start)!=1 or text.count(end)!=1: raise SystemExit(f'ambiguous production boundary: {start}')
    return text[text.index(start):text.index(end)]
parts=[section(text,'pub(super) fn kind(', 'fn initial('),
       section(text,'fn same_material(', 'fn lit('),
       section(text,'pub(super) fn tick_melter(', 'pub(super) fn casting_at(')]
(out/'runtime.rs').write_text('\n'.join(parts))
text=(root/'source-smeltery-fuel.rs').read_text()
(out/'capacity.rs').write_text(section(text,'pub(super) fn capacity(', '/// Portable tank items'))
PYEXTRACT
cd "$ROOT"
# Seed resolution with the pinned host's lockfile, adding only this local
# fixture package. Cargo metadata prunes unused packages while retaining the
# locked versions where possible; the strict comparison below checks that.
cp "$HORIZON_HOST/Cargo.lock" "$WORK/Cargo.lock"
cat >> "$WORK/Cargo.lock" <<'LOCK'

[[package]]
name = "melter-fuel-runtime-fixture"
version = "0.1.0"
dependencies = ["horizon-items", "horizon-world", "serde_json"]
LOCK
if ! cargo metadata --offline --manifest-path "$WORK/Cargo.toml" --format-version 1 > /dev/null; then
    echo 'Offline dependencies unavailable. Set CARGO_HOME to a populated cache for the pinned host.' >&2
    exit 1
fi
# The temporary package has its own lockfile. Reject any resolved package not
# present at the identical version/source/checksum in the pinned host lock.
python3 - <<'PYLOCK'
import os, tomllib
from pathlib import Path
host = tomllib.loads((Path(os.environ['HORIZON_HOST']) / 'Cargo.lock').read_text())
test = tomllib.loads((Path(os.environ['WORK']) / 'Cargo.lock').read_text())
def identity(p):
    return tuple(p.get(k) for k in ('name', 'version', 'source', 'checksum'))
allowed = {identity(p) for p in host['package']}
unknown = [identity(p) for p in test['package'] if p['name'] != 'melter-fuel-runtime-fixture' and identity(p) not in allowed]
if unknown:
    raise SystemExit(f'fixture dependencies differ from the pinned host lock: {unknown}')
PYLOCK
cargo test --locked --offline --manifest-path "$WORK/Cargo.toml" melter_fuel_tests:: -- --nocapture 2>&1 | tee "$WORK/results.log"
# Never accept a successful zero-test filter.
for name in liquid_driver_melts_controller_copper_with_exact_phase_budget heater_consumes_one_fuel_and_publishes_bucket_only_after_commit liquid_failure_is_atomic_and_completed_full_output_never_refuels no_recipe_low_temperature_and_short_burn_do_not_consume missing_structure_burns_stored_fuel_without_heating coal_driver_melts_without_draining_liquid_tank rejected_completion_preserves_live_item_and_pending_output; do
    grep -Fqx "test melter_fuel_tests::$name ... ok" "$WORK/results.log"
done
