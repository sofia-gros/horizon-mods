#!/usr/bin/env python3
"""Package a tracked Mixins bundle directory as a `.hnmod` (a native mod file for Horizon).

Layout: `horizon-native.json` at the root and the bundle under `mixins/<id>/`. Only what Horizon accepts in a bundle is
packed: `.rs` and `.toml` files, `data/` trees and README/LICENSE files; anything else tracked in the folder is an error,
so nothing unreviewed ships by accident.

  package_bundle.py BUNDLE_DIR OUTPUT.hnmod
  package_bundle.py BUNDLE_DIR OUTPUT.hnmod --manifest-inputs  # local working-tree development
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


def manifest_files(bundle: Path, config: dict) -> list[Path]:
    """Local development inputs selected by the manifest, independent of git staging."""
    selected = {bundle / 'horizon-mixin.toml'}

    def inside(name):
        rel = PurePosixPath(name)
        if not name or rel.is_absolute() or '..' in rel.parts or '\\' in name or ':' in name:
            raise ValueError(f'unsafe declared source path: {name}')
        path = bundle.joinpath(*rel.parts)
        for ancestor in (path, *path.parents):
            if ancestor == bundle.parent:
                break
            if ancestor.is_symlink():
                raise ValueError(f'symlink source: {name}')
        path.resolve().relative_to(bundle.resolve())
        return path

    for section in ('module', 'injection'):
        for entry in config.get(section, []):
            for key in ('source', 'helper'):
                if key in entry:
                    selected.add(inside(entry[key]))
    crates = list(config.get('crate', []))
    if 'api' in config:
        crates.append(config['api'])
    for crate in crates:
        folder = inside(crate['path'])
        selected.add(folder / 'Cargo.toml')
        src = folder / 'src'
        if not src.is_dir():
            raise ValueError(f'missing crate source directory: {src}')
        for path in src.rglob('*'):
            inside(path.relative_to(bundle).as_posix())
            if path.is_file():
                if path.suffix != '.rs':
                    raise ValueError(f'non-Rust crate input needs an explicit packaging contract: {path}')
                selected.add(path)
    # Data packs and licence notices are valid bundle inputs too.
    for path in bundle.rglob('*'):
        rel = path.relative_to(bundle)
        if 'target' in rel.parts:
            continue
        if path.is_file() and (rel.parts[0] == 'data' or path.name.startswith(DOC_PREFIXES)):
            inside(rel.as_posix())
            selected.add(path)
    return sorted(selected)


def build(bundle: Path, output: Path, root: Path = ROOT, *, manifest_inputs: bool = False) -> dict:
    manifest_path = bundle / 'horizon-mixin.toml'
    config = tomllib.loads(manifest_path.read_text(encoding='utf-8'))
    compat = json.loads((root / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    mod, pin = config['mod'], config['compatibility']['horizon_revision']
    if pin != compat['horizon_revision']:
        raise ValueError(f'{mod["id"]}: bundle pins Horizon {pin[:8]} but compatibility.json pins {compat["horizon_revision"][:8]}')
    manifest = {'schema': 1, 'id': mod['id'], 'version': mod['version'], 'game': '26.3',
                'horizon': pin, 'mixins': compat['mixins_revision']}
    sources = manifest_files(bundle, config) if manifest_inputs else tracked_files(bundle)
    # A working-tree manifest may already reference new files. Never report a
    # successful package while silently omitting untracked implementation code.
    tracked = set(sources)
    missing = sorted(p.relative_to(bundle).as_posix() for p in bundle.rglob('*')
                     if p.is_file() and p.suffix in ('.rs', '.toml')
                     and 'target' not in p.relative_to(bundle).parts and p not in tracked)
    if missing and not manifest_inputs:
        raise ValueError(f'{len(missing)} untracked source inputs would be omitted; review and track them before packaging: {", ".join(missing[:5])}')
    entries: dict[str, bytes] = {}
    for source in sources:
        if source.is_symlink() or not source.is_file():
            raise ValueError(f'not a regular tracked file: {source}')
        rel = PurePosixPath(source.relative_to(bundle).as_posix())
        if not allowed(rel):
            raise ValueError(f'{rel}: a native bundle may only carry .rs/.toml files, data/ and README/LICENSE files')
        entries[f'mixins/{mod["id"]}/{rel}'] = source.read_bytes()
    if f'mixins/{mod["id"]}/horizon-mixin.toml' not in entries:
        raise ValueError('horizon-mixin.toml is not tracked in the bundle')
    # Product-level notices travel with a bundle even when source lives in mod/.
    # Only tracked, direct parent notices are eligible; never sweep other mods.
    for source in tracked_files(bundle.parent):
        if source.parent == bundle.parent and source.name.startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE')):
            if source.is_symlink() or not source.is_file():
                raise ValueError(f'not a regular tracked notice: {source}')
            entries[source.name] = source.read_bytes()
    entries['horizon-native.json'] = (json.dumps(manifest, indent=2) + '\n').encode()
    if len(entries) > MAX_ENTRIES or sum(map(len, entries.values())) > MAX_UNPACKED:
        raise ValueError('bundle exceeds the native file limits (512 entries, 64 MiB)')
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'x') as archive:
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
    ap.add_argument('--manifest-inputs', action='store_true',
                    help='local development: include declared helpers and API/crate sources without git staging')
    a = ap.parse_args()
    try:
        print(json.dumps(build(a.bundle.resolve(), a.output, manifest_inputs=a.manifest_inputs), indent=2))
    except (ValueError, OSError) as error:
        sys.exit(f'package_bundle: {error}')
