#!/usr/bin/env python3
"""Export pinned Horizon sources and apply the pinned Mixins CLI."""
import argparse, json, subprocess, tarfile, tempfile
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
# Build only exported committed source, never a possibly modified working tree.
with tempfile.TemporaryDirectory(prefix='mixins-source-', dir=root / 'target') as temp:
    source = Path(temp)
    proc = subprocess.Popen(['git', '-C', str(a.mixins), 'archive', '--format=tar', compat['mixins_revision']], stdout=subprocess.PIPE)
    with tarfile.open(fileobj=proc.stdout, mode='r|') as archive:
        archive.extractall(source, filter='data')
    if proc.wait():
        raise SystemExit('Mixins git archive failed')
    cli_target = root / 'target/mixins-cli-build'
    subprocess.run(['cargo', 'build', '--locked', '-p', 'horizon-mixins-cli', '--target-dir', str(cli_target)], cwd=source, check=True)
    cli = cli_target / 'debug/horizon-mixins'
    subprocess.run([str(cli), 'apply', '--workspace', str(original), '--mods', str(root / 'mixins'), '--out', str(root / 'target/modded')], check=True)
