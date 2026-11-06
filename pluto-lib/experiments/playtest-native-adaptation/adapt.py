"""Pure mining-only bridge for two exact playtest inputs; chest hook remains unchanged."""
import base64, hashlib, json, tomllib, zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
TC = 'tconstruct/mixins/tconstruct/'
PC = 'pluto-lib/mixins/pluto_native_creative/'
MANIFEST = TC + 'horizon-mixin.toml'
COMMON = PC + 'horizon-mixin.toml'
CHEST = TC + 'source-chest-pick.rs'
MINING = TC + 'source-mining-context-native.rs'
FACTS = PC + 'mining-context.rs'
sha = lambda b: hashlib.sha256(b).hexdigest()

def oracle():
    return json.loads((HERE / 'oracles.json').read_text())

def originals(guide=False):
    o = oracle()
    data = {p: zlib.decompress(base64.b64decode(v, validate=True)) for p, v in o['original_sources_zlib_base64'].items()}
    if {p: sha(b) for p, b in data.items()} != o['source_sha256']:
        raise ValueError('Frozen input bytes drift')
    if guide:
        data[MANIFEST] = zlib.decompress(base64.b64decode(o['guide_postimage_zlib_base64'], validate=True))
        if sha(data[MANIFEST]) != o['guide_postimage_sha256']:
            raise ValueError('Guide postimage drift')
    return data

def replace_once(data, before, after):
    if data.count(before) != 1 or after in data:
        raise ValueError('Missing, repeated or already adapted block')
    result = data.replace(before, after, 1)
    if result.count(after) != 1 or result.replace(after, before, 1) != data:
        raise ValueError('Noninvertible replacement')
    return result

def revision(data, o, reverse=False):
    old = ('horizon_revision = "' + o['original_revision'] + '"').encode()
    new = ('horizon_revision = "' + o['official_revision'] + '"').encode()
    return replace_once(data, new, old) if reverse else replace_once(data, old, new)

def source(name, o):
    data = (HERE / name).read_bytes()
    if sha(data) != o['candidate_sha256'][name]:
        raise ValueError('Reviewed candidate source drift')
    return data

def restore_selected(after, guide=False):
    o = oracle(); expected = originals(guide)
    if set(after) != set(expected) | {MINING, FACTS}:
        raise ValueError('Wrong candidate selected set')
    out = dict(after)
    if out.pop(MINING) != source('source-mining-context-native.rs', o) or out.pop(FACTS) != source('mining-context.rs', o):
        raise ValueError('Added adapter bytes changed')
    out[MANIFEST] = replace_once(out[MANIFEST], o['mining_new_block'].encode(), o['mining_old_block'].encode())
    out[MANIFEST] = revision(out[MANIFEST], o, True)
    addition = o['common_new_block'].encode()
    if out[COMMON].count(addition) != 1 or not out[COMMON].endswith(addition):
        raise ValueError('Common appended owner changed')
    out[COMMON] = revision(out[COMMON][:-len(addition)], o, True)
    if out != expected:
        raise ValueError('Selected-source inverse failed')
    return out

def transform_selected(before, guide=False):
    """10 exact selected source files -> 12. This does not seal a repository/package."""
    o = oracle(); expected = originals(guide)
    if type(before) is not dict or before != expected:
        raise ValueError('Not an exact supported selected-source input')
    out = dict(before)
    out[MANIFEST] = revision(out[MANIFEST], o)
    out[MANIFEST] = replace_once(out[MANIFEST], o['mining_old_block'].encode(), o['mining_new_block'].encode())
    out[COMMON] = revision(out[COMMON], o)
    if not out[COMMON].endswith(b'\n') or b'pluto_mining_context' in out[COMMON]:
        raise ValueError('Common owner collision')
    out[COMMON] += o['common_new_block'].encode()
    out[MINING] = source('source-mining-context-native.rs', o)
    out[FACTS] = source('mining-context.rs', o)
    # Check the complete parsed TOML, not just presence of the desired declarations.
    want = tomllib.loads(before[MANIFEST].decode())
    want['compatibility']['horizon_revision'] = o['official_revision']
    hooks = want['injection']; old_ids = ['source-host-mining-context', 'source-client-mining-context']
    positions = [i for i, h in enumerate(hooks) if h['id'] in old_ids]
    if len(positions) != 2 or positions[1] != positions[0] + 1 or [hooks[i]['id'] for i in positions] != old_ids:
        raise ValueError('Old mining hook order changed')
    hooks[positions[0]:positions[1]+1] = tomllib.loads(o['mining_new_block'])['injection']
    if tomllib.loads(out[MANIFEST].decode()) != want:
        raise ValueError('Unrelated TConstruct semantics changed')
    want = tomllib.loads(before[COMMON].decode()); want['compatibility']['horizon_revision'] = o['official_revision']
    added = tomllib.loads(o['common_new_block'])['module'][0]
    if any(m['id'] == added['id'] or (m['file'], m['name']) == (added['file'], added['name']) for m in want['module']):
        raise ValueError('Existing common module owner collision')
    want['module'].append(added)
    if tomllib.loads(out[COMMON].decode()) != want:
        raise ValueError('Unrelated common semantics changed')
    if {p for p in before if out[p] != before[p]} != {MANIFEST, COMMON} or restore_selected(out, guide) != before:
        raise ValueError('Wrong scope or inverse')
    return out

def receipt(before, after, guide=False):
    if transform_selected(before, guide) != after:
        raise ValueError('Not the exact transformed result')
    return {'schema': 1, 'input_kind': 'PR945 plus Guide B/C postimage' if guide else 'PR945/e128 selected sources',
            'before': {p: sha(b) for p, b in sorted(before.items())},
            'after': {p: sha(b) for p, b in sorted(after.items())},
            'changed': sorted(p for p in before if before[p] != after[p]),
            'added': sorted(set(after) - set(before)), 'selected_inverse_verified': True,
            'repository_sealed': False, 'package_sealed': False, 'plan_verified': False,
            'native_type_verified': False, 'compatibility': None}
