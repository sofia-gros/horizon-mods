#!/usr/bin/env python3
"""Build the modded Horizon client for the smoke gate (slice of TConstruct + PlutoLib).

Pipeline: pinned upstream check -> package/extract PlutoLib -> slice both
bundles (tools/slice_manifest.py) -> stage.py (Mixins plan + PlutoLib guards for
the kept features) -> cargo build client+ctl -> prepare TConstruct resources.
Caller sets TMP/TEMP/CARGO_HOME/CARGO_TARGET_DIR if the default drive is small.
"""
import argparse, json, shutil, subprocess, sys
from pathlib import Path

root = Path(__file__).resolve().parents[2]
py = sys.executable
p = argparse.ArgumentParser()
p.add_argument('--horizon', type=Path, required=True, help='checkout at compatibility.json horizon_revision')
p.add_argument('--mixins', type=Path, required=True, help='checkout at compatibility.json mixins_revision')
p.add_argument('--base-resources', type=Path, help='prepared Horizon 26.3 resource folder (required unless --ci)')
p.add_argument('--slice-config', type=Path, default=Path(__file__).with_name('slice.json'), help='slice definition (default: slice.json)')
p.add_argument('--ci', action='store_true', help='stage + cargo check only: no client build, no resources (CI has no Minecraft assets)')
p.add_argument('--work', type=Path, default=root / '.local/smoke-build')
p.add_argument('--target-dir', type=Path, default=root / '.local/cargo-target-modded')
a = p.parse_args()
if not a.ci and not a.base_resources:
    p.error('--base-resources is required unless --ci')
compat = json.loads((root / 'tconstruct/compatibility.json').read_text())
cfg = json.loads(a.slice_config.read_text())

def run(*cmd, **kw):
    print('+', ' '.join(map(str, cmd)), flush=True)
    subprocess.run([str(c) for c in cmd], check=True, **kw)

def head(path):
    return subprocess.run(['git', '-C', str(path), 'rev-parse', 'HEAD'], capture_output=True, text=True, check=True).stdout.strip()

for name, path, key in (('horizon', a.horizon, 'horizon_revision'), ('mixins', a.mixins, 'mixins_revision')):
    if head(path) != compat[key]:
        sys.exit(f'BUILD FAIL: {name} checkout {head(path)} != pinned {compat[key]}')

if a.work.exists():
    shutil.rmtree(a.work)
a.work.mkdir(parents=True)
stage_out = root / 'tconstruct/target'
for stale in ('original', 'mixins-source', 'modded', 'complete-plan.json', 'mixins-revision.txt'):
    t = stage_out / stale
    shutil.rmtree(t) if t.is_dir() else t.unlink(missing_ok=True)

hnmod = a.work / 'plutolib.hnmod'
run(py, root / 'pluto-lib/tools/package_native.py', hnmod)
run(py, root / 'tconstruct/tools/extract_pluto.py', '--horizon', a.horizon, '--package', hnmod, '--output', a.work / 'pluto-installed')
full = a.work / 'pluto-installed/pluto_native_creative/mixins/pluto_native_creative'
pluto_slice = a.work / 'pluto-slice'
args = [py, root / 'tools/slice_manifest.py', '--out', a.work / 'mods/tconstruct', '--tokens-out', a.work / 'tconstruct-tokens.txt']
for s in cfg.get('tconstruct_seeds', []): args += ['--seed', s]
if 'tconstruct_all_except' in cfg:
    for x in (cfg['tconstruct_all_except'] or ['NONE']): args += ['--all-except', x]
run(*args)
# TConstruct sources call PlutoLib modules by name; keep those too.
import tomllib
pluto_names = {m['name'] for m in tomllib.loads((full / 'horizon-mixin.toml').read_text(encoding='utf-8'))['module']}
needed = sorted(pluto_names & set((a.work / 'tconstruct-tokens.txt').read_text().split()))
print('PlutoLib modules used by the TConstruct slice:', needed, flush=True)
args = [py, root / 'tools/slice_manifest.py', '--src', full, '--out', pluto_slice]
for s in cfg['pluto_seeds']: args += ['--seed', s]
for s in cfg['pluto_keep_prefixes']: args += ['--keep-prefix', s]
for n in needed: args += ['--keep-module-name', n]
run(*args)
run(py, root / 'tconstruct/tools/stage.py', '--horizon', a.horizon, '--mixins', a.mixins, '--mods', a.work / 'mods',
    '--pluto-bundle', pluto_slice, '--pluto-guards', ','.join(cfg['pluto_guards']))
manifest = stage_out / 'modded/Cargo.toml'
if a.ci:
    run('cargo', 'check', '--locked', '--manifest-path', manifest, '-p', 'horizon-client', '--bin', 'horizon-client', '--no-default-features')
    print(json.dumps({'result': 'CI CHECK OK (stage + cargo check only; not launched)'}))
    sys.exit(0)
env = dict(__import__('os').environ, CARGO_TARGET_DIR=str(a.target_dir))
run('cargo', 'build', '--manifest-path', manifest, '-p', 'horizon-client', '--bin', 'horizon-client', '--no-default-features', '--profile', 'fast', env=env)
run('cargo', 'build', '--manifest-path', manifest, '-p', 'horizon-client-lib', '--bin', 'horizon-ctl', '--no-default-features', '--profile', 'fast', env=env)
game = a.target_dir / 'fast/horizon-client.exe'
if not game.is_file(): game = a.target_dir / 'fast/horizon-client'
overlay, resources = a.work / 'item-resources', a.work / 'game-resources'
run(py, root / 'tconstruct/tools/resources.py', a.base_resources, overlay)
run(game, '--tconstruct-prepare', overlay, root / 'tconstruct/content', resources)
run(py, root / 'tconstruct/tools/prepare_horizon_source_data.py', a.base_resources, resources / 'minecraft-original-pack')
run(py, root / 'tconstruct/tools/prepare_bundled_forge.py', resources / 'forge-original-pack')
print(json.dumps({'result': 'BUILD OK', 'game': str(game), 'ctl': str(game.with_name(game.name.replace('client', 'ctl'))), 'resources': str(resources)}, indent=2))
