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
WORK=$(mktemp -d "${TMPDIR:-/tmp}/broken-casting.XXXXXXXX")
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
import re
mixins=root / 'tconstruct/mixins/tconstruct'
original=root / 'tconstruct/tests/rust/source_seared_reinforcement.rs'
base=original.read_text()
# Resolve original source and fixture paths before moving into the temp crate.
base=re.sub(r'(path = \"|include_bytes!\(\"|include_str!\(\")([^\"]+)',lambda m:m[1]+str((original.parent/m[2]).resolve()),base)
fixture=(root / 'tconstruct/tests/rust/broken_casting_fixture.rs').read_text().replace('__MIXINS__',str(mixins)).replace('__FIXTURE__',str(root / 'tconstruct/tests/fixtures/broken_iron_helmet.json'))
def block(text,start):
    begin=text.index(start);brace=text.index('{',begin);depth=1;i=brace+1
    while depth:
        depth+=(text[i]=='{')-(text[i]=='}');i+=1
    return text[begin:i]
fixture=fixture.replace('// __HARVEST_ALIAS__',block((mixins/'source-harvest-tags.rs').read_text(),'pub(super) fn alias('))
station=(mixins/'source-station-world.rs').read_text()
for marker,start in [('ASSEMBLE','pub(super) fn assemble('),('ASSEMBLE_STATS','pub fn assemble_with_stats('),('HELMET','fn stock_iron_helmet_repair('),('REBUILD','pub(super) fn rebuild_material_swap(')]:
    body=block(station,start)
    if marker=='REBUILD':body+='\n'+block(station,'pub(super) fn rebuild_casting_material_swap(')+'\n'+block(station,'fn rebuild_material_swap_inner(')
    fixture=fixture.replace('// __'+marker+'__',body)
tx=(mixins/'transaction.rs').read_text()
merge=tx[tx.index('pub fn source_merge_material_stats('):tx.index('/// Source material lookup over a complete')]
fixture=fixture.replace('// __MERGERS__',merge).replace('// __SET_DAMAGE__',block(tx,'pub(super) fn fixed_set_damage('))
world=(mixins/'source-melter-world.rs').read_text()
fixture=fixture.replace('// __CANDIDATE__',block(world,'if stack.count == 1 && stack.item.starts_with("tconstruct:")'))
(work/'lib.rs').write_text(base+'\n'+fixture)
manifest = f'''[package]
name = "broken-casting-fixture"
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
name = "broken-casting-fixture"
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
unknown = [identity(p) for p in test['package'] if p['name'] != 'broken-casting-fixture' and identity(p) not in allowed]
if unknown:
    raise SystemExit(f'fixture dependencies differ from the pinned host lock: {unknown}')
PYLOCK
cargo test --locked --offline --manifest-path "$WORK/Cargo.toml" -- --nocapture 2>&1 | tee "$WORK/results.log"
# A successful zero-test filter is not evidence that the target behavior ran.
for case in \
    'test stock_helmet_requires_current_complete_pinned_modifier_definitions ... ok' \
    'test stock_broken_iron_helmet_casts_to_usable_steel_and_preserves_until_complete ... ok' \
    'test stock_exception_rejects_unknown_item_data_wire_and_unsupported_traits ... ok' \
    'test broken_composite_and_missing_stock_context_remain_closed ... ok' \
    'test repaired_damage_uses_new_maximum_and_rejects_inconsistent_flags ... ok'
do
    if ! grep -Fqx "$case" "$WORK/results.log"; then
        printf 'Required test did not pass: %s\n' "$case" >&2
        exit 1
    fi
done
