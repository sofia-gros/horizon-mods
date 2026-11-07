#!/usr/bin/env python3
"""Rewrite every pinned Horizon/Mixins revision to new full SHAs.

Touches only revision metadata: tconstruct/compatibility.json, pluto-lib/horizon-native.json
and every pluto-lib/tconstruct horizon-mixin.toml. It does NOT make the code
compatible; run tools/smoke-local/build.py afterwards (plan + guards + build).
Usage: bump_upstream.py --horizon SHA --mixins SHA
"""
import argparse, json, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHA = re.compile(r'^[0-9a-f]{40}$')


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('--horizon', required=True)
    ap.add_argument('--mixins', required=True)
    a = ap.parse_args(argv)
    for name in ('horizon', 'mixins'):
        if not SHA.match(getattr(a, name)):
            sys.exit(f'--{name} must be a full 40-character lowercase SHA')
    compat_path = ROOT / 'tconstruct/compatibility.json'
    compat = json.loads(compat_path.read_text())
    old = {'horizon': compat['horizon_revision'], 'mixins': compat['mixins_revision']}
    new = {'horizon': a.horizon, 'mixins': a.mixins}
    changed = 0
    files = [compat_path, ROOT / 'pluto-lib/horizon-native.json']
    files += sorted((ROOT / 'pluto-lib').rglob('horizon-mixin.toml'))
    files += sorted((ROOT / 'tconstruct/mixins').rglob('horizon-mixin.toml'))
    for path in files:
        if '.local' in path.parts or 'target' in path.parts:
            continue
        text = path.read_text(encoding='utf-8')
        updated = text.replace(old['horizon'], new['horizon']).replace(old['mixins'], new['mixins'])
        if updated != text:
            path.write_bytes(updated.encode('utf-8'))
            changed += 1
    print(f'bumped horizon {old["horizon"][:8]} -> {a.horizon[:8]}, mixins {old["mixins"][:8]} -> {a.mixins[:8]} in {changed} file(s)')


if __name__ == '__main__':
    main()
