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
FIXTURE=${MACHINE_GUARD_FIXTURE:-machine_destroy_guard.rs}
case "$FIXTURE" in machine_destroy_guard.rs|smeltery_nbt_store.rs) ;; *) echo 'Unsupported machine fixture' >&2; exit 1;; esac
export ROOT HORIZON_HOST FIXTURE
WORK=$(mktemp -d "${TMPDIR:-/tmp}/machine-destroy-guard.XXXXXXXX")
trap 'rm -rf "$WORK"' EXIT
export WORK
python3 - <<'PY'
import json, os
from pathlib import Path
host = Path(os.environ['HORIZON_HOST'])
root = Path(os.environ['ROOT'])
work = Path(os.environ['WORK'])
for crate in ('horizon-world', 'horizon-items', 'horizon-client-core'):
    if not (host / 'crates' / crate / 'Cargo.toml').is_file():
        raise SystemExit(f'missing host crate: {crate}')
def quoted(path):
    return json.dumps(str(path))
manifest = f'''[package]
name = "machine-destroy-guard-fixture"
version = "0.1.0"
edition = "2024"
[dependencies]
horizon-world = {{ path = {quoted(host / 'crates/horizon-world')} }}
horizon-items = {{ path = {quoted(host / 'crates/horizon-items')} }}
horizon-client-core = {{ path = {quoted(host / 'crates/horizon-client-core')} }}
[lib]
path = {quoted(root / 'tconstruct/tests/rust' / os.environ['FIXTURE'])}
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
name = "machine-destroy-guard-fixture"
version = "0.1.0"
dependencies = ["horizon-client-core", "horizon-items", "horizon-world"]
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
unknown = [identity(p) for p in test['package'] if p['name'] != 'machine-destroy-guard-fixture' and identity(p) not in allowed]
if unknown:
    raise SystemExit(f'fixture dependencies differ from the pinned host lock: {unknown}')
PYLOCK
cargo test --locked --offline --manifest-path "$WORK/Cargo.toml" -- --nocapture 2>&1 | tee "$WORK/results.log"
# A successful zero-test filter is not evidence that the target behavior ran.
if [[ "$FIXTURE" == smeltery_nbt_store.rs ]]; then
    cases=('test write_many_rejects_wrong_destination_without_mutating_first_chunk ... ok')
else
    cases=('test verified_close_allows_filled_controller_after_page_offset_is_discarded ... ok'
           'test close_never_erases_mismatched_or_unknown_page ... ok'
           'test forged_valid_shaped_snapshot_does_not_authorize_diamond_deletion ... ok'
           'test writer_cannot_erase_unexplained_or_retained_wire_live_items ... ok'
           'test guest_pickup_final_write_failure_rolls_back_deposit_withdrawal_and_outside_drop ... ok'
           'test guest_pickup_success_commits_source_before_acknowledging ... ok')
fi
for case in "${cases[@]}"; do
    if ! grep -Fqx "$case" "$WORK/results.log"; then
        printf 'Required test did not pass: %s\n' "$case" >&2
        exit 1
    fi
done
