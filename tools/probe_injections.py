#!/usr/bin/env python3
"""Ask the pinned Mixins CLI which TConstruct injections the current Horizon accepts.

For every injection id in the full manifest, build a one-injection slice
(tools/slice_manifest.py) and run `horizon-mixins plan` on the pristine
workspace. Planning checks that the target exists and the kind is supported; it
does not compile helpers, so an accepted injection can still fail `cargo check`.

  probe_injections.py --cli CLI --workspace ORIGINAL --out result.json [--jobs 6]
"""
import argparse, json, re, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import slice_deps as sd


def probe(args, ident):
    with tempfile.TemporaryDirectory(prefix='probe-', dir=args.tmp) as tmp:
        mods = Path(tmp) / 'mods'
        r = subprocess.run([sys.executable, str(ROOT / 'tools/slice_manifest.py'), '--seed', ident, '--out', str(mods / 'tconstruct')],
                           capture_output=True, text=True)
        if r.returncode:
            return ident, {'status': 'tool-error', 'message': (r.stderr or r.stdout)[-300:]}
        p = subprocess.run([str(args.cli), 'plan', '--workspace', str(args.workspace), '--mods', str(mods), '--out', str(Path(tmp) / 'plan.json')],
                           capture_output=True, text=True)
        if p.returncode == 0:
            return ident, {'status': 'accepted'}
        m = re.search(r'\{.*\}', p.stdout + p.stderr, re.S)
        try:
            err = json.loads(m.group(0))
        except Exception:
            err = {'message': (p.stdout + p.stderr)[-300:]}
        return ident, {'status': 'rejected', 'code': err.get('code'), 'message': err.get('message'), 'target': err.get('target')}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--cli', type=Path, required=True)
    ap.add_argument('--workspace', type=Path, required=True)
    ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--jobs', type=int, default=6)
    ap.add_argument('--tmp', type=Path, default=None, help='scratch directory (default: system temp)')
    a = ap.parse_args()
    ids = list(sd.inj)
    result = {}
    with ThreadPoolExecutor(a.jobs) as pool:
        for n, (ident, res) in enumerate(pool.map(lambda i: probe(a, i), ids), 1):
            result[ident] = res
            print(f'{n}/{len(ids)} {ident}: {res["status"]}', flush=True)
            a.out.write_text(json.dumps(result, indent=1), encoding='utf-8', newline='\n')
    counts = {}
    for v in result.values():
        counts[v['status']] = counts.get(v['status'], 0) + 1
    print(counts)


if __name__ == '__main__':
    main()
