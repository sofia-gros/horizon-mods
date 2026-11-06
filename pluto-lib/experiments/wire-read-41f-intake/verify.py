"""Read-only exact historical full-plan check; no subprocess or native execution."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def check(plan_path, official, mod_root):
    proof = json.loads((HERE / 'receipt.json').read_text())
    before, after = proof['before'], proof['after']
    assert set(before) == set(after)
    assert {k for k in before if before[k] != after[k]} == {'horizon_revision', 'file_sha256'}
    restored = dict(after)
    for key in ('horizon_revision', 'file_sha256'):
        restored[key] = before[key]
    assert sha((json.dumps(restored, indent=2) + '\n').encode()) == proof['before_sha256']
    assert sha((json.dumps(after, indent=2) + '\n').encode()) == proof['after_sha256']
    raw = Path(plan_path).read_bytes()
    assert len(raw) == proof['plan_bytes'] and sha(raw) == proof['plan_sha256']
    guard = Path(mod_root) / 'pluto-lib/tools/check_wire_snapshot_plan.py'
    assert sha(guard.read_bytes()) == proof['guard_sha256']
    for name, expected in proof['module_sources_sha256'].items():
        assert sha((Path(mod_root) / 'pluto-lib/mixins/pluto_native_creative' / name).read_bytes()) == expected
    spec = importlib.util.spec_from_file_location('fixed_wire_checker', guard)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.check(json.loads(raw), Path(official), after)


if __name__ == '__main__':
    if len(sys.argv) != 4:
        raise SystemExit('Expected PLAN OFFICIAL_ROOT MOD_ROOT')
    check(*sys.argv[1:])
    print('Exact full-plan Wire guard and two-field metadata inverse verified; native unrun.')
