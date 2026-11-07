"""Restore one pinned generated float/range token boundary, never pristine source.

No Rust parser is implemented here. The exact textual method boundary and known
plan closure are checked; the subsequent ordinary Cargo build is the syntax gate.
"""
import hashlib
import json
import os
from pathlib import Path
import stat
import tempfile

HORIZON = '41f822b659789780f58d32c63ddfb6887ea1b8bf'
MIXINS = '5a64af938ec86b00d53d16a408aabc0038fb3e57'
TARGET = 'crates/horizon-world/src/worldgen/structure/start.rs'
ORIGINAL_SHA = 'ccb0e6ac41d7af0cd893ca5cfc4f659ca9c0b8a2ad17f5148fd70451ed38352d'
HORIZON_LOCK = '0bc4234f3001d44df9d46fbb06df331d3f53b1af6b613ec9082baaec3d93c62f'
MIXINS_PINS = {
    'Cargo.lock': '35846f4a621cf8e1e54f6f0f7e13ac848db590d4ec401620501c107954eae6ca',
    'crates/horizon-mixins-transform/src/lib.rs': 'b70b3aa2d8e95fabfefa4374bb63d5e87fb959871fd3bb92e5c3317e71d9ced1',
    'crates/horizon-mixins-core/src/native.rs': '97a4728b5416d3c5dfa5e34b2e4e1e1c495940a8f72c45323a94adc3031bcd85',
}
PRETTYPLEASE_VERSION = '0.2.37'
PRETTYPLEASE_CHECKSUM = '479ca8adacdd7ce8f1fb39ce9ecccbfe93a3f1344b3d0d97f20bc0196208f62b'
CLOSURE_SHA = '73b6aa778881de3f5c1cad01640be4172d2b597546bc4ee6c793d565c102fe2b'
RECEIPT = 'float-range-repair-receipt.json'
MAX_FILE = 64 * 1024**2
MAX_STAGE = 2 * 1024**3
MAX_FILES = 20000
BAD = b'if !(0...=1.).contains(&frequency) {'
GOOD = b'if !(0. ..=1.).contains(&frequency) {'
SIGNATURE = b'''impl StructureRegistry {
    pub fn new(
        structures: &BTreeMap<String, String>,
        sets: &BTreeMap<String, String>,
        pools: BTreeMap<String, String>,
        biome_set: &dyn Fn(&Value) -> Result<BTreeSet<BiomeId>, DecodeError>,
    ) -> Result<Self, DecodeError> {'''
NEXT = b'    pub fn dimension_origin(&self) -> ChunkPos {'
TAIL = b'''        Ok(Self {
            structures: defs,
            sets: out,
            pools: Pools::new(pools),
            origin: ChunkPos::new(0, 0),
        })
    }'''
CONTEXT = b'''            let frequency = p["frequency"].as_f64().unwrap_or(1.);
            if !(0...=1.).contains(&frequency) {
                return Err(DecodeError::Field("structure set frequency"));
            }
            let placement = match p["type"].as_str().unwrap_or_default() {'''


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':')).encode()


def plain(path, directory=False):
    path = Path(path)
    for item in (path, *path.parents):
        if item.is_symlink() or getattr(item, 'is_junction', lambda: False)():
            raise ValueError('Linked repair input')
    mode = path.lstat().st_mode
    if not (stat.S_ISDIR(mode) if directory else stat.S_ISREG(mode)):
        raise ValueError('Unexpected repair input kind')
    return path


def read(path, limit=MAX_FILE):
    path = plain(path)
    before = path.stat()
    if not 0 <= before.st_size <= limit:
        raise ValueError('Repair input exceeds bound')
    data = path.read_bytes()
    after = path.stat()
    if (before.st_size, before.st_mtime_ns, before.st_ino, before.st_mode) != (after.st_size, after.st_mtime_ns, after.st_ino, after.st_mode) or len(data) != before.st_size:
        raise ValueError('Repair input changed during read')
    return data


def snapshot(root):
    root = plain(root, True)
    result = {}; total = 0
    for parent, dirs, files in os.walk(root, followlinks=False):
        for name in sorted(dirs + files):
            path = Path(parent) / name
            relative = path.relative_to(root).as_posix()
            if name in dirs:
                plain(path, True)
                result[relative] = ['directory', stat.S_IMODE(path.stat().st_mode)]
            else:
                data = read(path)
                total += len(data)
                if total > MAX_STAGE:
                    raise ValueError('Repair stage exceeds total budget')
                result[relative] = ['file', stat.S_IMODE(path.stat().st_mode), len(data), sha(data)]
            if len(result) > MAX_FILES:
                raise ValueError('Repair stage exceeds entry budget')
    return result


def candidate(data):
    """Exact known anchors only; no broad replacement or claims of Rust AST parsing."""
    if data.count(BAD) != 1 or data.count(GOOD) != 0:
        raise ValueError('Expected exactly one bad spelling and no corrected spelling')
    eol = b'\r\n' if b'\r\n' in data else b'\n'
    if eol == b'\r\n' and b'\n' in data.replace(b'\r\n', b''):
        raise ValueError('Mixed generated line endings')
    signature, following, tail, context = (p.replace(b'\n', eol) for p in (SIGNATURE, NEXT, TAIL, CONTEXT))
    if any(data.count(p) != 1 for p in (signature, following, tail, context)):
        raise ValueError('Known method/context is absent or not unique')
    begin = data.index(signature); end = data.index(following)
    at = data.index(BAD); tail_at = data.index(tail)
    if not begin < at < tail_at < end or end - begin > 64 * 1024:
        raise ValueError('Known expression is outside the bounded method')
    middle = data[begin + len(signature):end]
    if any(mark in middle for mark in (eol+b'    pub fn ', eol+b'    fn ', eol+b'}')):
        raise ValueError('Unexpected intervening method or impl boundary')
    if not begin < data.index(context) < tail_at:
        raise ValueError('Known context belongs to a different method')
    offset = at + len(b'if !(0.')
    after = data[:offset] + b' ' + data[offset:]
    if after.count(GOOD) != 1 or after.count(BAD) != 0 or after[:offset] + after[offset+1:] != data:
        raise ValueError('Single-byte inverse failed')
    return after, offset


def verify_plan(plan, original):
    if plan.get('schema') != 6 or plan.get('inputs', {}).get(TARGET) != sha(original) or plan.get('inputs', {}).get('Cargo.lock') != HORIZON_LOCK:
        raise ValueError('Repair plan does not describe exact pristine inputs')
    closure = {key: [row for row in plan.get(key, []) if row.get('file') == TARGET or any(c.get('file') == TARGET for c in row.get('constructors', []))]
               for key in ('injections', 'modules', 'states', 'bridges')}
    if sha(canonical(closure)) != CLOSURE_SHA:
        raise ValueError('Existing target-file patch closure differs')
    return {'sha256': CLOSURE_SHA, 'injections': 1, 'modules': 9, 'states': 0, 'bridges': 0}


def repair(original, modded, mixins_source, plan_path, *, horizon_revision, mixins_revision):
    """Call only after successful ordinary complete-plan guards and official apply."""
    if (horizon_revision, mixins_revision) != (HORIZON, MIXINS):
        raise ValueError('Unsupported repair revision')
    roots = [plain(p, True).resolve(strict=True) for p in (original, modded, mixins_source)]
    if any(a == b or a.is_relative_to(b) or b.is_relative_to(a) for i, a in enumerate(roots) for b in roots[i+1:]):
        raise ValueError('Repair roots must be disjoint')
    original, modded, mixins_source = roots
    receipt_path = modded.parent / RECEIPT
    if receipt_path.exists() or receipt_path.is_symlink():
        raise ValueError('Repair receipt is not fresh')
    pin_paths = {original/TARGET: ORIGINAL_SHA, original/'Cargo.lock': HORIZON_LOCK,
                 **{mixins_source/p: digest for p, digest in MIXINS_PINS.items()}}
    for path, digest in pin_paths.items():
        if sha(read(path)) != digest:
            raise ValueError('Pristine official or serializer source pin differs')
    pristine = read(original/TARGET)
    if pristine.count(GOOD) != 1 or BAD in pristine:
        raise ValueError('Pristine expression is not the known valid spelling')
    plan_raw = read(plan_path, 16*1024**2)
    closure = verify_plan(json.loads(plan_raw), pristine)
    before_map = snapshot(modded)
    target = plain(modded/TARGET)
    before = read(target)
    after, offset = candidate(before)
    mode = stat.S_IMODE(target.stat().st_mode)
    # All admission checks precede the single generated-file replacement.
    if snapshot(modded) != before_map:
        raise ValueError('Stage changed before repair')
    pending = None
    try:
        with tempfile.NamedTemporaryFile(prefix='.float-range-', dir=target.parent, delete=False) as stream:
            pending = Path(stream.name); stream.write(after); stream.flush(); os.fsync(stream.fileno())
        os.chmod(pending, mode)
        os.replace(pending, target); pending = None
    finally:
        if pending is not None:
            pending.unlink(missing_ok=True)
    if read(target) != after or after[:offset] + after[offset+1:] != before:
        raise ValueError('Generated repair readback/inverse differs')
    expected = dict(before_map); expected[TARGET] = ['file', mode, len(after), sha(after)]
    if snapshot(modded) != expected:
        raise ValueError('Other stage bytes, modes, entries or locks changed')
    if read(plan_path, 16*1024**2) != plan_raw or any(sha(read(p)) != digest for p, digest in pin_paths.items()):
        raise ValueError('Plan or pristine inputs changed')
    result = {'schema': 1, 'kind': 'known-float-range-token-boundary', 'target': TARGET,
              'horizon': HORIZON, 'mixins': MIXINS, 'prettyplease': PRETTYPLEASE_VERSION,
              'prettyplease_checksum': PRETTYPLEASE_CHECKSUM, 'method': 'StructureRegistry::new',
              'before_sha256': sha(before), 'after_sha256': sha(after),
              'before_bytes': len(before), 'after_bytes': len(after), 'inserted_bytes': 1,
              'offset': offset, 'inverse_verified': True, 'other_stage_entries_unchanged': True,
              'original_and_locks_unchanged': True, 'plan_sha256': sha(plan_raw), 'patch_closure': closure,
              'stage_before_seal_sha256': sha(canonical(before_map)), 'stage_after_seal_sha256': sha(canonical(expected)),
              'scope_check': 'Exact textual impl/signature/next-member/tail/context; not an AST parser',
              'ast_verified': False, 'syntax_gate': 'Subsequent ordinary Cargo build, not yet implied by this receipt',
              'semantic_change': False, 'repair_applied': True}
    raw = json.dumps(result, sort_keys=True, indent=2).encode() + b'\n'
    if len(raw) > 8192:
        raise ValueError('Repair receipt exceeds bound')
    with receipt_path.open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    if read(receipt_path, 8192) != raw:
        raise ValueError('Repair receipt readback differs')
    return result
