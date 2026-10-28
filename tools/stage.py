#!/usr/bin/env python3
"""Export pinned Horizon sources and apply the pinned Mixins CLI."""
import argparse, json, subprocess, tarfile
from pathlib import Path

root = Path(__file__).resolve().parents[1]
compat = json.loads((root / 'compatibility.json').read_text())
p = argparse.ArgumentParser()
p.add_argument('--horizon', type=Path, required=True)
p.add_argument('--mixins', type=Path, required=True)
a = p.parse_args()
for repo, key in [(a.horizon, 'horizon_revision'), (a.mixins, 'mixins_revision')]:
    head = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip()
    if head != compat[key]:
        p.error(f'{repo}: expected {compat[key]}, found {head}')
original = root / 'target/original'
if original.exists():
    p.error(f'{original} already exists; use a fresh checkout or explicitly remove this generated export')
original.mkdir(parents=True)
proc = subprocess.Popen(['git', '-C', str(a.horizon), 'archive', '--format=tar', compat['horizon_revision']], stdout=subprocess.PIPE)
with tarfile.open(fileobj=proc.stdout, mode='r|') as archive:
    archive.extractall(original, filter='data')
if proc.wait():
    raise SystemExit('git archive failed')
for path in original.rglob('*'):
    if path.is_symlink():
        data = path.read_bytes()
        path.unlink()
        path.write_bytes(data)
(original / '.horizon-revision').write_text(compat['horizon_revision'] + '\n')
subprocess.run(['cargo', 'build', '--locked', '-p', 'horizon-mixins-cli'], cwd=a.mixins, check=True)
cli = a.mixins.resolve() / 'target/debug/horizon-mixins'
subprocess.run([str(cli), 'apply', '--workspace', str(original), '--mods', str(root / 'mixins'), '--out', str(root / 'target/modded')], check=True)
