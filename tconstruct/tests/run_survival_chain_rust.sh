#!/usr/bin/env bash
# Compile the bounded source fixture against a compatible local Horizon host.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
: "${HORIZON_HOST:?Set HORIZON_HOST to a local Horizon 624c4f62 checkout or staged workspace}"
HORIZON_HOST=$(cd "$HORIZON_HOST" && pwd)
EXPECTED=624c4f6265c02296fbe8a80f029dd0d987fb94c0
ACTUAL=$(git -C "$HORIZON_HOST" rev-parse HEAD 2>/dev/null) || {
    echo 'HORIZON_HOST must be a verifiable Git checkout of the pinned host.' >&2; exit 1;
}
if [[ "$ACTUAL" != "$EXPECTED" ]] || [[ -n "$(git -C "$HORIZON_HOST" status --porcelain --untracked-files=no)" ]]; then
    echo "HORIZON_HOST must have a clean tracked tree at $EXPECTED." >&2; exit 1
fi
export ROOT HORIZON_HOST
WORK=$(mktemp -d "${TMPDIR:-/tmp}/survival-chain.XXXXXXXX")
trap 'rm -rf "$WORK"' EXIT
export WORK
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
name = "survival-chain-fixture"
version = "0.1.0"
edition = "2024"
[dependencies]
horizon-world = {{ path = {quoted(host / 'crates/horizon-world')} }}
horizon-items = {{ path = {quoted(host / 'crates/horizon-items')} }}
serde_json = "1"
[lib]
path = {quoted(root / 'tconstruct/tests/rust/survival_helper_chain.rs')}
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
cd "$ROOT"
# Seed resolution with the pinned host's lockfile, adding only this local
# fixture package. Cargo metadata prunes unused packages while retaining the
# locked versions where possible; the strict comparison below checks that.
cp "$HORIZON_HOST/Cargo.lock" "$WORK/Cargo.lock"
cat >> "$WORK/Cargo.lock" <<'LOCK'

[[package]]
name = "survival-chain-fixture"
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
unknown = [identity(p) for p in test['package'] if p['name'] != 'survival-chain-fixture' and identity(p) not in allowed]
if unknown:
    raise SystemExit(f'fixture dependencies differ from the pinned host lock: {unknown}')
PYLOCK
cargo test --locked --offline --manifest-path "$WORK/Cargo.toml" survival_chain_tests:: -- --nocapture 2>&1 | tee "$WORK/results.log"
# Never accept a successful zero-test filter.
for name in copper_controller_and_rose_gold_chain_retain_quantities rejected_heat_alloy_and_cast_do_not_consume; do
    grep -Fqx "test survival_chain_tests::$name ... ok" "$WORK/results.log"
done
