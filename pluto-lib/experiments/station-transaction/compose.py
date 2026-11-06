"""Source-only proposal; no native execution, writes, or manifest activation."""
import hashlib

BASE = '17f44dc36fc2df28547328aa8554286ae965b208'
OFFICIAL = '41f822b659789780f58d32c63ddfb6887ea1b8bf'
POINT = '    let key = (pos[0], pos[1], pos[2]);\n'
INSERT = '    crate::game::recipe_book::tconstruct_station_transaction::ensure();\n'

def sha(value):
    return hashlib.sha256(value.encode()).hexdigest()

def compose(inputs, oracles, candidate):
    if inputs['base'] != BASE or inputs['official'] != OFFICIAL:
        raise ValueError('Wrong source cutoff')
    if any(inputs[k] for k in ['runtime_installed', 'old_hook_removed', 'native_executed']):
        raise ValueError('Unproved activation')
    for values, expected in [(oracles, inputs['oracle_sha256']), (candidate, inputs['candidate_sha256'])]:
        if set(values) != set(expected) or any(sha(v) != expected[k] for k, v in values.items()):
            raise ValueError('Input set or source hash differs')
    station = oracles['station.rs']
    if station.count(POINT) != 1 or INSERT in station:
        raise ValueError('Registration point is not unique')
    changed = station.replace(POINT, INSERT + POINT)
    if changed.replace(INSERT, '') != station:
        raise ValueError('Station inverse differs')
    prefix = candidate['consumer-prefix.rs']
    if not prefix.endswith('\n'):
        raise ValueError('Consumer prefix boundary differs')
    consumer = prefix + oracles['old-transaction.rs']
    if consumer[len(prefix):] != oracles['old-transaction.rs']:
        raise ValueError('Transaction body differs')
    files = {
        'source-crafting-station.rs': changed,
        'station-transaction-consumer.rs': consumer,
        'recipe-transaction.rs': candidate['native-transaction.rs'],
        'recipe-placement-click.rs': candidate['single-head.rs'],
    }
    return files, {
        'files': {k: {'sha256': sha(v), 'bytes': len(v.encode())} for k, v in files.items()},
        'station_inverse_exact': True,
        'legacy_transaction_body_exact': True,
        'ordinary_manifest_changed': False,
        'old_hook_removed': False,
        'native_executed': False,
    }

def verify_native(inputs, oracles, source):
    if sha(source) != inputs['official_recipe_book_sha256']:
        raise ValueError('Official recipe_book source differs')
    if source.count(oracles['official/place_recipe']) != 1:
        raise ValueError('Native function boundary differs')
