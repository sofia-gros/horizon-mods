#!/usr/bin/env bash
# Compile the bounded source fixture against a compatible local Horizon host.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
: "${HORIZON_HOST:?Set HORIZON_HOST to a local Horizon ebcfa226 checkout or staged workspace}"
HORIZON_HOST=$(cd "$HORIZON_HOST" && pwd)
EXPECTED=ebcfa2263aa3d8e399aa993b1e05daf42eb4cfa0
ACTUAL=$(git -C "$HORIZON_HOST" rev-parse HEAD 2>/dev/null) || {
    echo 'HORIZON_HOST must be a verifiable Git checkout of the pinned host.' >&2; exit 1;
}
if [[ "$ACTUAL" != "$EXPECTED" ]] || [[ -n "$(git -C "$HORIZON_HOST" status --porcelain --untracked-files=no)" ]]; then
    echo "HORIZON_HOST must have a clean tracked tree at $EXPECTED." >&2; exit 1
fi
export ROOT HORIZON_HOST
WORK=$(mktemp -d "${TMPDIR:-/tmp}/fern-loot.XXXXXXXX")
trap 'rm -rf "$WORK"' EXIT
export WORK
python3 - <<'PY'
import json, os
from pathlib import Path
host = Path(os.environ['HORIZON_HOST'])
root = Path(os.environ['ROOT'])
work = Path(os.environ['WORK'])
for crate in ('horizon-core', 'horizon-world', 'horizon-items'):
    if not (host / 'crates' / crate / 'Cargo.toml').is_file():
        raise SystemExit(f'missing host crate: {crate}')
def quoted(path):
    return json.dumps(str(path))
manifest = f'''[package]
name = "fern-loot-fixture"
version = "0.1.0"
edition = "2024"
[dependencies]
horizon-core = {{ path = {quoted(host / 'crates/horizon-core')} }}
horizon-world = {{ path = {quoted(host / 'crates/horizon-world')} }}
horizon-items = {{ path = {quoted(host / 'crates/horizon-items')} }}
serde_json = "1"
[lib]
path = {quoted(root / 'tconstruct/tests/rust/nether_slime_fern_loot.rs')}
'''
(work / 'Cargo.toml').write_text(manifest)
PY
cd "$ROOT"
# Seed resolution with the pinned host's lockfile, adding only this local
# fixture package. Cargo metadata prunes unused packages while retaining the
# locked versions where possible; the strict comparison below checks that.
cp "$HORIZON_HOST/Cargo.lock" "$WORK/Cargo.lock"
cat >> "$WORK/Cargo.lock" <<'LOCK'

[[package]]
name = "fern-loot-fixture"
version = "0.1.0"
dependencies = ["horizon-core", "horizon-items", "horizon-world", "serde_json"]
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
unknown = [identity(p) for p in test['package'] if p['name'] != 'fern-loot-fixture' and identity(p) not in allowed]
if unknown:
    raise SystemExit(f'fixture dependencies differ from the pinned host lock: {unknown}')
PYLOCK
FERN_FIXTURE_ROOT="$ROOT/tconstruct" cargo test --locked --offline --manifest-path "$WORK/Cargo.toml" -- --nocapture 2>&1 | tee "$WORK/results.log"
# A successful zero-test filter is not evidence that the target behavior ran.
case='test nether_ferns_drop_without_shears_other_ferns_do_not ... ok'
if ! grep -Fqx "$case" "$WORK/results.log"; then
    printf 'Required test did not pass: %s\n' "$case" >&2
    exit 1
fi
case='test nether_fern_explosion_condition_is_evaluated_by_host ... ok'
if ! grep -Fqx "$case" "$WORK/results.log"; then
    printf 'Required test did not pass: %s\n' "$case" >&2
    exit 1
fi
