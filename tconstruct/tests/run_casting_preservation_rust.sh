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
WORK=$(mktemp -d "${TMPDIR:-/tmp}/casting-preservation.XXXXXXXX")
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
# Compile the actual full interaction functions with bounded host fixtures.
base=(root / 'tconstruct/tests/rust/smeltery_nbt_store.rs').read_text()
base=base.replace('../../mixins/tconstruct/', str(root / 'tconstruct/mixins/tconstruct') + '/')
base=base.replace('pub chunks: HashMap<(i32,i32), Arc<Chunk>>, pub modified:', 'pub blocks: HashMap<(i32,i32,i32), u8>, pub block_entities: HashMap<(i32,i32,i32), super::BlockEntity>, pub chunks: super::Chunks, pub modified:')
route=(root / 'tconstruct/mixins/tconstruct/source-machine-runtime.rs').read_text()
fixture=(root / 'tconstruct/tests/rust/casting_interaction_fixture.rs').read_text()
fixture=fixture.replace('// __CASTING_AT__',route[route.index('pub(super) fn casting_at('):route.index('// Empty-cast items')])
fixture=fixture.replace('// __INTERACT_CASTING__',route[route.index('pub(super) fn interact_casting('):route.index('\nfn faucet_neighbors(')])
(work / 'lib.rs').write_text(base+'\n'+fixture)
manifest = f'''[package]
name = "casting-preservation-fixture"
version = "0.1.0"
edition = "2024"
[dependencies]
horizon-world = {{ path = {quoted(host / 'crates/horizon-world')} }}
horizon-items = {{ path = {quoted(host / 'crates/horizon-items')} }}
serde_json = "1"
[lib]
path = {quoted(work / 'lib.rs')}
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
name = "casting-preservation-fixture"
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
unknown = [identity(p) for p in test['package'] if p['name'] != 'casting-preservation-fixture' and identity(p) not in allowed]
if unknown:
    raise SystemExit(f'fixture dependencies differ from the pinned host lock: {unknown}')
PYLOCK
cargo test --locked --offline --manifest-path "$WORK/Cargo.toml" -- --nocapture 2>&1 | tee "$WORK/results.log"
# A successful zero-test filter is not evidence that the target behavior ran.
for case in \
    'test actual_casting_failed_fluid_probe_rechecks_before_all_item_fallbacks ... ok' \
    'test actual_casting_unchanged_probe_preserves_all_item_fallbacks ... ok' \
    'test actual_casting_held_transfers_preserve_extensions_in_all_four_blocks ... ok' \
    'test actual_casting_held_transfer_rechecks_live_saved_block_and_player_snapshots ... ok' \
    'test actual_casting_held_transfer_rejects_aliases_stale_slots_and_failed_save ... ok' \
    'test actual_casting_interaction_rejects_wire_insertion_molding_and_live_slots ... ok' \
    'test actual_casting_interaction_accepts_normal_insertion_and_reusable_molding ... ok' \
    'test typed_casting_write_rejects_live_wire_before_encoding_and_slot_patch_is_safe ... ok' \
    'test casting_cooling_and_completion_preserve_native_extensions ... ok' \
    'test unchanged_opaque_items_survive_ticks_but_consumption_aborts_batch ... ok' \
    'test casting_rejects_ambiguous_aliases_and_lossy_ids_but_snapshot_replaces ... ok'
do
    if ! grep -Fqx "$case" "$WORK/results.log"; then
        printf 'Required test did not pass: %s\n' "$case" >&2
        exit 1
    fi
done
