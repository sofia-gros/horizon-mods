#!/usr/bin/env python3
"""Package the tracked native bundle and its separate resource preparation inputs.

The nested data archive is inert launcher data, not an automatically installed
content mod. See the README for the required resource-preparation steps.
"""
import argparse
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[2]
MOD = ROOT / 'tconstruct'
MIXINS = MOD / 'mixins' / 'tconstruct'
MAX_ENTRIES = 512
MAX_UNPACKED = 64 * 1024 * 1024


def tracked(*paths):
    output = subprocess.check_output(['git', '-C', str(ROOT), 'ls-files', '-z', '--', *paths])
    for raw in output.split(b'\0'):
        if not raw:
            continue
        rel = Path(raw.decode('utf-8'))
        source = ROOT / rel
        if source.is_symlink() or not source.is_file():
            raise ValueError(f'not a regular tracked file: {rel}')
        yield rel, source


def sliced_manifest():
    """The Tinkers manifest without the injections the pinned Horizon Mixins refuses (tools/smoke-local/slice.json).

    The official launcher drops a whole mod when Mixins rejects any of its hooks, so shipping the full manifest would ship nothing."""
    config = json.loads((ROOT / 'tools/smoke-local/slice.json').read_text(encoding='utf-8'))
    command = [sys.executable, str(ROOT / 'tools/slice_manifest.py')]
    for ident in config['tconstruct_all_except']:
        command += ['--all-except', ident]
    # Mixins re-prints edited files and breaks `0. ..=1.` in these Horizon files; the launcher cannot work around it
    # (our own stage.py does), so nothing that edits them ships until Mixins is fixed.
    for host_file in config.get('launcher_unsafe_files', []):
        command += ['--avoid-file', host_file]
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / 'tconstruct'
        subprocess.run(command + ['--out', str(out)], check=True, capture_output=True)
        return (out / 'horizon-mixin.toml').read_bytes()


def write_zip_file(archive, name, payload):
    info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
    info.compress_type = zipfile.ZIP_DEFLATED
    info.external_attr = 0o100644 << 16
    archive.writestr(info, payload)


def package(output):
    config = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text(encoding='utf-8'))
    compatibility = json.loads((MOD / 'compatibility.json').read_text(encoding='utf-8'))
    mod = config['mod']
    horizon = config['compatibility']['horizon_revision']
    if mod['id'] != 'tconstruct' or horizon != compatibility['horizon_revision']:
        raise ValueError('native bundle identity or pinned Horizon revision differs')
    # The content companion owns tconstruct; use a distinct native ID so the
    # launcher does not suppress the content mod when native code is installed.
    native_id = 'tconstruct_native'
    manifest = {'schema': 1, 'id': native_id, 'version': mod['version'], 'game': '26.3',
                'horizon': horizon, 'mixins': compatibility['mixins_revision']}

    # Keep the thousands of original asset files inside one permitted data
    # entry; retain the paths expected by the existing preparation scripts.
    support = io.BytesIO()
    support_paths = ('tconstruct/content', 'tconstruct/assets', 'tconstruct/third_party',
                     'tconstruct/docs', 'tconstruct/tools', 'pluto-lib/pluto_lib')
    with zipfile.ZipFile(support, 'w') as archive:
        for rel, source in tracked(*support_paths):
            if rel.parts[:2] == ('tconstruct', 'docs') and not rel.name.endswith('.sha256'):
                continue
            if rel.parts[:2] == ('tconstruct', 'tools') and source.suffix != '.py':
                continue
            write_zip_file(archive, rel.as_posix(), source.read_bytes())

    files = [('horizon-native.json', (json.dumps(manifest, indent=2) + '\n').encode())]
    manifest_payload = sliced_manifest()
    sliced = tomllib.loads(manifest_payload.decode('utf-8'))
    # Do not ship orphaned root helpers of excluded injections. Keep every API/crate
    # subtree intact; only manifest-addressed root sources are subject to this filter.
    roots = {entry[key] for kind in ('module', 'injection') for entry in sliced.get(kind, [])
             for key in ('source', 'helper') if key in entry}
    for rel, source in tracked('tconstruct/mixins/tconstruct'):
        inside = rel.relative_to('tconstruct/mixins/tconstruct')
        if len(inside.parts) == 1 and inside.suffix == '.rs' and inside.as_posix() not in roots:
            continue
        if source.suffix not in ('.rs', '.toml') and source.name != 'Cargo.lock':
            raise ValueError(f'unsupported native source: {rel}')
        # Generated accessors are namespaced by the native mod ID.
        payload = (manifest_payload if inside.as_posix() == 'horizon-mixin.toml' else source.read_bytes()).replace(
            b'__horizon_mixins_tconstruct_', b'__horizon_mixins_tconstruct_native_')
        if inside.as_posix() == 'horizon-mixin.toml':
            payload = payload.replace(b'id = "tconstruct"', b'id = "tconstruct_native"', 1)
        files.append((f'mixins/{native_id}/{inside.as_posix()}', payload))
    files.append((f'mixins/{native_id}/data/tconstruct-support.zip', support.getvalue()))
    if len(files) > MAX_ENTRIES or sum(len(data) for _, data in files) > MAX_UNPACKED:
        raise ValueError('native bundle exceeds the official entry or size limit')
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'w') as archive:
        for name, data in files:
            write_zip_file(archive, name, data)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    package(parser.parse_args().output)
