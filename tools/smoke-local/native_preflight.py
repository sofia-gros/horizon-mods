"""Guard the exact installed native-probe composition before launcher execution.

Only the temporary CLI-view headers omit trait metadata unsupported by the old
planner. The launcher's real bundles keep their API/dependency declarations.
"""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]


def cli_manifest(text):
    kept, skipping = [], False
    for line in text.splitlines(keepends=True):
        header = line.strip()
        if header.startswith('['):
            skipping = header in ('[api]', '[dependencies]')
        if not skipping:
            kept.append(line)
    return ''.join(kept)


def has_operations(document):
    return any(document.get(section) for section in ('injection', 'bridge', 'module', 'state', 'crate', 'post_build'))


def check(cli, workspace, bundles, work, guards, horizon_revision, mixins_revision, log=None):
    cli, workspace, bundles, work = map(Path, (cli, workspace, bundles, work))
    if not cli.is_file() or not workspace.is_dir():
        raise ValueError('native preflight requires the built pinned Mixins CLI and an official Horizon checkout')
    head = subprocess.run(['git', '-C', str(workspace), 'rev-parse', 'HEAD'],
                          capture_output=True, text=True, check=True).stdout.strip()
    if head != horizon_revision:
        raise ValueError('native preflight Horizon checkout differs from the native-wrapper pin')
    revision_file = ROOT / 'tconstruct/target/mixins-revision.txt'
    if not revision_file.is_file() or revision_file.read_text(encoding='utf-8').strip() != mixins_revision:
        raise ValueError('run the guarded staging build with the pinned Mixins before native preflight')
    plan = work / 'native-preflight-plan.json'
    api_only = []
    with tempfile.TemporaryDirectory(prefix='native-preflight-', dir=work) as temp:
        combined = Path(temp) / 'mods'
        combined.mkdir()
        for bundle in sorted(bundles.iterdir()):
            if not bundle.is_dir():
                continue
            manifests = ([bundle / 'horizon-mixin.toml'] if (bundle / 'horizon-mixin.toml').is_file()
                         else list(bundle.glob('mixins/*/horizon-mixin.toml')))
            if len(manifests) != 1:
                raise ValueError(f'{bundle.name}: expected exactly one raw mod manifest')
            original = manifests[0].read_text(encoding='utf-8')
            document = tomllib.loads(original)
            compatible = cli_manifest(original)
            if not has_operations(tomllib.loads(compatible)):
                # The old CLI rejects empty mechanical bundles. API-only mods
                # contribute no Mixin operation, but remain in the real native
                # build for linkage and the exact-mod report check.
                api = document.get('api', {}).get('path')
                if not isinstance(api, str):
                    raise ValueError(f'{bundle.name}: empty mod without an API package')
                package = (manifests[0].parent / api).resolve()
                if not package.is_relative_to(manifests[0].parent.resolve()) or not (package / 'Cargo.toml').is_file():
                    raise ValueError(f'{bundle.name}: missing or escaping API package')
                api_only.append(bundle.name)
                continue
            destination = combined / bundle.name
            shutil.copytree(manifests[0].parent, destination)
            manifest = destination / 'horizon-mixin.toml'
            manifest.write_text(compatible, encoding='utf-8', newline='\n')
        subprocess.run([str(cli.resolve()), 'plan', '--workspace', str(workspace.resolve()),
                        '--mods', str(combined), '--out', str(plan.resolve())], check=True, stdout=log, stderr=subprocess.STDOUT)
        # All current core bundles carry the structure adapters. The sliced-plan
        # guard independently rejects an omitted structures guard when present.
        names = sorted(set(guards) | {'structures'})
        subprocess.run([sys.executable, str(ROOT / 'pluto-lib/tools/check_sliced_plan.py'),
                        str(plan), str(workspace.resolve()), ','.join(names)], check=True, stdout=log, stderr=subprocess.STDOUT)
    print(json.dumps({'result': 'NATIVE PROBE PREFLIGHT PASS', 'plan': str(plan), 'guards': names, 'api_only_linkage_pending': api_only}), file=log or sys.stdout, flush=True)
