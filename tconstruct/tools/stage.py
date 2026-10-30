#!/usr/bin/env python3
"""Export pinned Horizon sources and apply a resolved Mixins revision."""
import argparse, json, os, re, subprocess
from portable_archive import extract_git_tar
from pathlib import Path

root = Path(__file__).resolve().parents[1]
compat = json.loads((root / 'compatibility.json').read_text())
p = argparse.ArgumentParser()
p.add_argument('--horizon', type=Path, required=True)
p.add_argument('--mixins', type=Path, required=True)
p.add_argument('--mixins-revision', help='full Mixins commit SHA for a reproducible build; defaults to the latest official revision')
p.add_argument('--mods', type=Path, default=root / 'mixins', help='directory of Mixins bundles (defaults to this branch’s TConstruct bundle)')
p.add_argument('--output-dir', type=Path, default=root / 'target', help='fresh output directory')
a = p.parse_args()
head = subprocess.check_output(['git', '-C', str(a.horizon), 'rev-parse', 'HEAD'], text=True).strip()
if head != compat['horizon_revision']:
    p.error(f'{a.horizon}: expected {compat["horizon_revision"]}, found {head}')
if a.mixins_revision:
    if not re.fullmatch(r'[0-9a-f]{40}', a.mixins_revision):
        p.error('--mixins-revision must be a full lowercase commit SHA')
    revision = subprocess.check_output(['git', '-C', str(a.mixins), 'rev-parse', '--verify', a.mixins_revision + '^{commit}'], text=True).strip()
else:
    subprocess.run(['git', '-C', str(a.mixins), 'fetch', '--no-tags', compat['mixins_repository'], 'HEAD'], check=True)
    revision = subprocess.check_output(['git', '-C', str(a.mixins), 'rev-parse', '--verify', 'FETCH_HEAD^{commit}'], text=True).strip()
print(f'Horizon Mixins revision: {revision}', flush=True)
output = a.output_dir.resolve()
output.mkdir(parents=True, exist_ok=True)
mods = a.mods.resolve(strict=True)
original = output / 'original'
source = output / 'mixins-source'
if original.exists() or (output / 'modded').exists() or source.exists():
    p.error(f'{output}: original, modded or mixins-source already exists; choose a fresh output directory')
original.mkdir(parents=True)
proc = subprocess.Popen(['git', '-C', str(a.horizon), 'archive', '--format=tar', compat['horizon_revision']], stdout=subprocess.PIPE)
extract_git_tar(proc.stdout, original)
if proc.wait():
    raise SystemExit('git archive failed')
(original / '.horizon-revision').write_text(compat['horizon_revision'] + '\n')
# Build only exported committed source, never a possibly modified working tree.
source.mkdir()
proc = subprocess.Popen(['git', '-C', str(a.mixins), 'archive', '--format=tar', revision], stdout=subprocess.PIPE)
extract_git_tar(proc.stdout, source)
if proc.wait():
    raise SystemExit('Mixins git archive failed')
(output / 'mixins-revision.txt').write_text(revision + '\n')
cli_target = root.parent / 'target/mixins-cli-build'
subprocess.run(['cargo', 'build', '--release', '--locked', '-p', 'horizon-mixins-cli', '--target-dir', str(cli_target)], cwd=source, check=True)
cli = cli_target / 'release' / ('horizon-mixins.exe' if os.name == 'nt' else 'horizon-mixins')
subprocess.run([str(cli), 'apply', '--workspace', str(original), '--mods', str(mods), '--out', str(output / 'modded')], check=True)
