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
WORK=$(mktemp -d "${TMPDIR:-/tmp}/selected-fuel.XXXXXXXX")
trap 'rm -rf "$WORK"' EXIT
export WORK
export SELECTED_FUEL_EXTRACT="$WORK/extracted"
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
name = "selected-fuel-fixture"
version = "0.1.0"
edition = "2024"
[dependencies]
horizon-world = {{ path = {quoted(host / 'crates/horizon-world')} }}
horizon-items = {{ path = {quoted(host / 'crates/horizon-items')} }}
serde_json = "1"
[lib]
path = {quoted(root / 'tconstruct/tests/rust/smeltery_selected_fuel.rs')}
'''
(work / 'Cargo.toml').write_text(manifest)
PY
export SELECTED_FUEL_RECIPES="$WORK/recipes"
python3 - <<'PYRECIPES'
import os, zipfile
from pathlib import Path
root = Path(os.environ['ROOT'])
out = Path(os.environ['SELECTED_FUEL_RECIPES'])
out.mkdir()
with zipfile.ZipFile(root / 'tconstruct/content/reference/tconstruct-recipes.zip') as archive:
    for path in archive.namelist():
        if path.startswith('data/tconstruct/recipes/smeltery/melting/fuel/') and path.endswith('.json'):
            # Flat names prevent archive paths from becoming filesystem paths.
            (out / (path.rsplit('/', 1)[1])).write_bytes(archive.read(path))
PYRECIPES
python3 - <<'PYEXTRACT'
import os
from pathlib import Path
root=Path(os.environ['ROOT'])/'tconstruct/mixins/tconstruct'
out=Path(os.environ['SELECTED_FUEL_EXTRACT']); out.mkdir()
text=(root/'source-machine-runtime.rs').read_text()
def section(text,start,end):
    if text.count(start)!=1 or text.count(end)!=1: raise SystemExit(f'ambiguous production boundary: {start}')
    return text[text.index(start):text.index(end)]
parts=[section(text,'fn alloy_snapshot(', 'pub(super) fn kind('),
       section(text,'pub(super) fn kind(', 'fn initial(')]
(out/'helpers.rs').write_text('\n'.join(parts))
start=text.index('    let mut fuel_update = None;')
end=text.index('    if state.phase == 1 {',start)
(out/'selection.rs').write_text('{\n'+text[start:end]+'\nfuel_update\n}')
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
name = "selected-fuel-fixture"
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
unknown = [identity(p) for p in test['package'] if p['name'] != 'selected-fuel-fixture' and identity(p) not in allowed]
if unknown:
    raise SystemExit(f'fixture dependencies differ from the pinned host lock: {unknown}')
PYLOCK
cargo test --locked --offline --manifest-path "$WORK/Cargo.toml" selected_fuel_tests:: -- --nocapture 2>&1 | tee "$WORK/results.log"
# Never accept a successful zero-test filter.
for name in cold_selected_fuel_never_debits_hotter_later_tank missing_insufficient_or_zero_temperature_fuel_allows_fallback loaded_fuel_and_other_phases_do_not_select_again new_snapshot_after_reload_changes_selection_without_cached_temperature selected_fuel_patch_preserves_extensions_and_is_only_staged unsupported_tagged_or_malformed_fuel_stays_untouched_and_allows_fallback; do
    grep -Fqx "test selected_fuel_tests::$name ... ok" "$WORK/results.log"
done
