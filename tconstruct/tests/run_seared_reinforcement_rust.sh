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
WORK=$(mktemp -d "${TMPDIR:-/tmp}/seared-reinforcement.XXXXXXXX")
trap 'rm -rf "$WORK"' EXIT
export WORK
python3 - <<'PY'
import json, os
from pathlib import Path
host = Path(os.environ['HORIZON_HOST'])
root = Path(os.environ['ROOT'])
import sys
sys.path.insert(0, str(root / 'tconstruct/tests'))
from fixture_pluto_dependency import add_inventory_api
work = Path(os.environ['WORK'])
for crate in ('horizon-world', 'horizon-items'):
    if not (host / 'crates' / crate / 'Cargo.toml').is_file():
        raise SystemExit(f'missing host crate: {crate}')
def quoted(path):
    return json.dumps(str(path))
manifest = f'''[package]
name = "seared-reinforcement-fixture"
version = "0.1.0"
edition = "2024"
[dependencies]
horizon-world = {{ path = {quoted(host / 'crates/horizon-world')} }}
horizon-items = {{ path = {quoted(host / 'crates/horizon-items')} }}
serde_json = "1"
[lib]
path = {quoted(root / 'tconstruct/tests/rust/source_seared_reinforcement.rs')}
'''
(work / 'Cargo.toml').write_text(manifest)
add_inventory_api(work / 'Cargo.toml', root)
PY
cd "$ROOT"
# Seed resolution with the pinned host's lockfile, adding only this local
# fixture package. Cargo metadata prunes unused packages while retaining the
# locked versions where possible; the strict comparison below checks that.
cp "$HORIZON_HOST/Cargo.lock" "$WORK/Cargo.lock"
cat >> "$WORK/Cargo.lock" <<'LOCK'

[[package]]
name = "seared-reinforcement-fixture"
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
import sys
root = Path(os.environ['ROOT'])
sys.path.insert(0, str(root / 'tconstruct/tests'))
from fixture_pluto_dependency import inventory_api_identity
allowed.add(inventory_api_identity(root))
unknown = [identity(p) for p in test['package'] if p['name'] != 'seared-reinforcement-fixture' and identity(p) not in allowed]
if unknown:
    raise SystemExit(f'fixture dependencies differ from the pinned host lock: {unknown}')
PYLOCK
cargo test --locked --offline --manifest-path "$WORK/Cargo.toml" tests:: -- --nocapture 2>&1 | tee "$WORK/results.log"
# A successful zero-test filter is not evidence that the target behavior ran.
for case in \
    'test tests::remaining_reinforcements_cast_only_with_registered_output_and_consume_pattern ... ok' \
    'test tests::water_bottles_cast_with_potion_components_and_survive_state_roundtrip ... ok' \
    'test tests::water_result_parser_rejects_unknown_nbt_and_ambiguous_plain_potion ... ok' \
    'test tests::pinned_recipe_casts_with_each_fluid_and_consumes_pattern ... ok' \
    'test tests::invalid_and_different_amount_alternatives_fail_closed ... ok'
do
    if ! grep -Fqx "$case" "$WORK/results.log"; then
        printf 'Required test did not pass: %s\n' "$case" >&2
        exit 1
    fi
done
