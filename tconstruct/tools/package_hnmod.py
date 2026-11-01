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
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[2]
MOD = ROOT / 'tconstruct'
MIXINS = MOD / 'mixins' / 'tconstruct'
# The exact Mixins revision used for the last integrated Linux validation.
MIXINS_REVISION = 'ddd9700b21997c95d95ac6ee5ecafe3830cf622a'
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
    manifest = {'schema': 1, 'id': mod['id'], 'version': mod['version'], 'game': '26.3',
                'horizon': horizon, 'mixins': MIXINS_REVISION}

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
    for rel, source in tracked('tconstruct/mixins/tconstruct'):
        inside = rel.relative_to('tconstruct/mixins/tconstruct')
        if source.suffix not in ('.rs', '.toml'):
            raise ValueError(f'unsupported native source: {rel}')
        files.append((f'mixins/tconstruct/{inside.as_posix()}', source.read_bytes()))
    files.append(('mixins/tconstruct/data/tconstruct-support.zip', support.getvalue()))
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
