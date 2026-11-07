"""Pure, exact-base PR700 intake. Returns postimages; never writes files or refs."""
from pathlib import Path
import hashlib
import json

ROOT = Path(__file__).resolve().parent
ALLOWED = {
    'tconstruct/mixins/tconstruct/source-machine-runtime.rs',
    'tconstruct/tests/rust/faucet_tank_casting_fixture.rs',
    'tconstruct/tests/run_faucet_tank_casting.py',
    'tconstruct/docs/smeltery.md',
}

def sha(data):
    return hashlib.sha256(data).hexdigest()

def transform(data, change):
    if change['path'] not in ALLOWED or sha(data) != change['before_sha256']:
        raise ValueError('unexpected path or before bytes')
    lines = data.decode().splitlines(keepends=True)
    previous = len(lines) + 1
    for hunk in reversed(change['hunks']):
        start = hunk['start_line']
        old = hunk['old'].splitlines(keepends=True)
        if not isinstance(start, int) or not 0 <= start < previous or lines[start:start + len(old)] != old:
            raise ValueError('invalid or overlapping exact hunk')
        previous = start
        lines[start:start + len(old)] = hunk['new'].splitlines(keepends=True)
    result = ''.join(lines).encode()
    if sha(result) != change['after_sha256']:
        raise ValueError('unexpected postimage')
    return result

def build(root):
    root = Path(root)
    document = json.loads((ROOT / 'intake.json').read_text())
    if {entry['path'] for entry in document['changes']} != ALLOWED or len(document['changes']) != len(ALLOWED):
        raise ValueError('unexpected intake set')
    api = document['shared_api']
    if sha((root / api['path']).read_bytes()) != api['sha256']:
        raise ValueError('shared API drift')
    manifest = (root / 'pluto-lib/mixins/pluto_native_creative/horizon-mixin.toml').read_text()
    if manifest.count(api['declaration']) != 1:
        raise ValueError('shared API owner/declaration drift')
    return {entry['path']: transform((root / entry['path']).read_bytes(), entry) for entry in document['changes']}
