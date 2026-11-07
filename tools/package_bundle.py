#!/usr/bin/env python3
"""Package a tracked Mixins bundle directory as a `.hnmod` (a native mod file for Horizon).

Layout: `horizon-native.json` at the root and the bundle under `mixins/<id>/`. Only what Horizon accepts in a bundle is
packed: `.rs` and `.toml` files, `data/` trees and README/LICENSE files; anything else tracked in the folder is an error,
so nothing unreviewed ships by accident.

  package_bundle.py BUNDLE_DIR OUTPUT.hnmod
"""
import argparse
import json
import subprocess
import sys
import tomllib
import zipfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
MAX_ENTRIES = 512
MAX_UNPACKED = 64 * 1024 * 1024
DOC_PREFIXES = ('README', 'LICENSE', 'LICENCE', 'COPYING', 'NOTICE')


def allowed(rel: PurePosixPath) -> bool:
    if rel.suffix in ('.rs', '.toml'):
        return True
    if rel.parts[0] == 'data' or 'data' in rel.parts[1:2]:
        return True
    return rel.name.startswith(DOC_PREFIXES) or rel.name == '.gitignore'


def tracked_files(bundle: Path) -> list[Path]:
    out = subprocess.check_output(['git', '-C', str(ROOT), 'ls-files', '-z', '--', str(bundle.relative_to(ROOT))])
    return [ROOT / raw.decode('utf-8') for raw in out.split(b'\0') if raw]


def build(bundle: Path, output: Path, root: Path = ROOT) -> dict:
    manifest_path = bundle / 'horizon-mixin.toml'
    config = tomllib.loads(manifest_path.read_text(encoding='utf-8'))
    compat = json.loads((root / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    mod, pin = config['mod'], config['compatibility']['horizon_revision']
    if pin != compat['horizon_revision']:
        raise ValueError(f'{mod["id"]}: bundle pins Horizon {pin[:8]} but compatibility.json pins {compat["horizon_revision"][:8]}')
    manifest = {'schema': 1, 'id': mod['id'], 'version': mod['version'], 'game': '26.3',
                'horizon': pin, 'mixins': compat['mixins_revision']}
    entries: dict[str, bytes] = {}
    for source in tracked_files(bundle):
        if source.is_symlink() or not source.is_file():
            raise ValueError(f'not a regular tracked file: {source}')
        rel = PurePosixPath(source.relative_to(bundle).as_posix())
        if not allowed(rel):
            raise ValueError(f'{rel}: a native bundle may only carry .rs/.toml files, data/ and README/LICENSE files')
        entries[f'mixins/{mod["id"]}/{rel}'] = source.read_bytes()
    if f'mixins/{mod["id"]}/horizon-mixin.toml' not in entries:
        raise ValueError('horizon-mixin.toml is not tracked in the bundle')
    entries['horizon-native.json'] = (json.dumps(manifest, indent=2) + '\n').encode()
    if len(entries) > MAX_ENTRIES or sum(map(len, entries.values())) > MAX_UNPACKED:
        raise ValueError('bundle exceeds the native file limits (512 entries, 64 MiB)')
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'w') as archive:
        for name in sorted(entries):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, entries[name])
    return manifest


if __name__ == '__main__':
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('bundle', type=Path)
    ap.add_argument('output', type=Path)
    a = ap.parse_args()
    try:
        print(json.dumps(build(a.bundle.resolve(), a.output), indent=2))
    except (ValueError, OSError) as error:
        sys.exit(f'package_bundle: {error}')
