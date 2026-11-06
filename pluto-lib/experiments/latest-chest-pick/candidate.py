"""Two-path, unapplied owned-result bridge; no new Pluto API or native execution."""
import hashlib, json, tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
MANIFEST = 'tconstruct_native/horizon-mixin.toml'
HELPER = 'tconstruct_native/source-chest-pick.rs'
HOOK = 'tinkers-chest-pick-color'
BASE_MEMBERS_SHA256 = '02882fbd01e9dc5f06a3cd61d53ae9497dcabfd8302ebf98bba99892d9ceb823'
sha = lambda b: hashlib.sha256(b).hexdigest()

def oracle():
    return json.loads((HERE / 'oracles.json').read_text())

def seal(payload):
    return sha(json.dumps({p: sha(b) for p, b in sorted(payload.items())}, sort_keys=True).encode())

def transform_manifest(before):
    o = oracle()
    if sha(before) != o['base_manifest_sha256']:
        raise ValueError('Not the exact b78 full646 manifest')
    old, new = o['old_hook'].encode(), o['new_hook'].encode()
    if before.count(old) != 1 or new in before:
        raise ValueError('Ambiguous original hook')
    after = before.replace(old, new, 1)
    expected = tomllib.loads(before.decode())
    actual = tomllib.loads(after.decode())
    hooks = expected['injection']
    positions = [i for i, h in enumerate(hooks) if h['id'] == HOOK]
    if len(positions) != 1:
        raise ValueError('Hook owner not unique')
    hooks[positions[0]] = tomllib.loads(new.decode())['injection'][0]
    if expected != actual or after.replace(new, old, 1) != before:
        raise ValueError('Non-hook change or failed inverse')
    return after

def transform_helper(before):
    o = oracle()
    if before != o['old_helper'].encode() or sha(before) != o['old_helper_sha256']:
        raise ValueError('Original helper changed')
    old_header, new_header = o['old_header'].encode(), o['new_header'].encode()
    old_binding, new_binding = o['old_binding'].encode(), o['new_binding'].encode()
    if before.count(old_header) != 1 or before.count(old_binding) != 1:
        raise ValueError('Ambiguous helper edits')
    after = before.replace(old_header, new_header, 1).replace(old_binding, new_binding, 1)
    if after != (HERE / 'source-chest-pick.rs').read_bytes():
        raise ValueError('Candidate helper changed')
    inverse = after.replace(new_header, old_header, 1).replace(new_binding, old_binding, 1)
    if inverse != before:
        raise ValueError('Helper inverse failed')
    # The item guard, exact reader expression, Tag::Int handling and mutation are verbatim.
    anchor = b'    if &*stack.item'
    if before[before.index(anchor):] != after[after.index(anchor):]:
        raise ValueError('Consumer policy tail changed')
    return after

def restore_payload(after):
    o = oracle()
    out = after.copy()
    new, old = o['new_hook'].encode(), o['old_hook'].encode()
    if out[MANIFEST].count(new) != 1:
        raise ValueError('Candidate hook missing or repeated')
    out[MANIFEST] = out[MANIFEST].replace(new, old, 1)
    expected = transform_helper(o['old_helper'].encode())
    if out[HELPER] != expected:
        raise ValueError('Candidate helper drift')
    out[HELPER] = o['old_helper'].encode()
    if len(out) != 646 or seal(out) != BASE_MEMBERS_SHA256:
        raise ValueError('Not a complete inverse to the previous full646 payload')
    return out

def transform_payload(before):
    if not isinstance(before, dict) or any(not isinstance(p, str) or not isinstance(b, bytes) for p, b in before.items()):
        raise ValueError('Expected immutable payload bytes')
    if len(before) != 646 or seal(before) != BASE_MEMBERS_SHA256:
        raise ValueError('Wrong full646 input')
    after = before.copy()
    after[MANIFEST] = transform_manifest(before[MANIFEST])
    after[HELPER] = transform_helper(before[HELPER])
    changes = {p for p in before if after[p] != before[p]}
    if changes != {MANIFEST, HELPER} or restore_payload(after) != before:
        raise ValueError('Wrong changes or full inverse')
    return after

def verify_official(path):
    data = Path(path).read_bytes()
    o = oracle()
    if sha(data) != o['official_block_items_sha256']:
        raise ValueError('Official picked_stack source drift')
    method = o['official_method'].encode()
    if data.count(method) != 1 or not method.endswith(b'        Some(stack)\n    }'):
        raise ValueError('Final owned result site changed')
    if method.count(b'let mut stack = ItemStack::new(item, 1);') != 1:
        raise ValueError('Stack declaration changed')
    return {'source_verified': True, 'static_loads': o['static_loads'],
            'actual_ir_verified': False, 'native_type_verified': False}
