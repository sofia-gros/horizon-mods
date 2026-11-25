#!/usr/bin/env python3
"""Guard the exact native read-only Wire child module and additive comparator."""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
MODULES = {
    'wire-read': ('crates/horizon-items/src/item_data.rs', 'pluto_wire_read', 'wire-read.rs'),
    'stack-snapshot': ('crates/horizon-client/src/client_lib.rs', 'pluto_stack_snapshot', 'stack-snapshot.rs'),
    'stack-storage': ('crates/horizon-client/src/client_lib.rs', 'pluto_stack_storage', 'stack-storage.rs'),
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def metadata():
    return json.loads((ROOT / 'tools/wire-read-source.json').read_text())


NATIVE_FILE = 'crates/horizon-items/src/item_data.rs'
NATIVE_SHA256 = '0198190a7faa886a67b9683218fa1b3e815f29c3f019c3f59d8bd168eee0f4ed'


def check(plan, workspace, meta=None):
    if any(entry.get('mod_id') == 'pluto_lib'
           for section in ('injections', 'modules', 'states') for entry in plan.get(section, [])):
        # The trait bundle never reads Wire's private tuple. Keep native
        # wire_patch validation/equality unchanged, with no additive comparator
        # or snapshot capability implied by accepting this plan.
        if plan.get('schema') != 6 or meta is not None:
            raise ValueError('unreviewed native Wire schema or metadata override')
        legacy_names = {name for _, name, _ in MODULES.values()}
        for section in ('injections', 'modules', 'states'):
            for entry in plan.get(section, []):
                if any(c.get('file') == NATIVE_FILE for c in entry.get('constructors', [])):
                    raise ValueError('state constructor changes a preserved native Wire source')
                if (entry.get('file') == NATIVE_FILE or entry.get('mod_id') == 'pluto_native_creative'
                        or entry.get('name') in legacy_names
                        or entry.get('target', '').startswith('sunrise_items::item_data::')):
                    raise ValueError('native Wire must remain unattached; no mixed private-field seam')
        root = Path(workspace)
        source = root / NATIVE_FILE
        if source.is_symlink() or not source.is_file():
            raise ValueError('native Wire source must be a regular file')
        source.resolve().relative_to(root.resolve())
        if plan.get('inputs', {}).get(NATIVE_FILE) != NATIVE_SHA256:
            raise ValueError('native Wire plan source differs')
        if digest(source.read_bytes()) != NATIVE_SHA256:
            raise ValueError('native Wire workspace source differs')
        return
    meta = metadata() if meta is None else meta
    source = (Path(workspace) / meta['file']).read_bytes()
    if digest(source) != meta['file_sha256'] or source.count(meta['declaration'].encode()) != 1:
        raise ValueError('Official private Wire declaration/source differs from the reviewed target')
    reader = (ROOT / 'mixins/pluto_native_creative/wire-read.rs').read_bytes()
    if digest(reader) != meta['reader_sha256']:
        raise ValueError('Read-only Wire adapter differs from the reviewed source')
    for identifier, (file, name, source_file) in MODULES.items():
        selected = [m for m in plan.get('modules', []) if m.get('file') == file and m.get('name') == name]
        if len(selected) != 1:
            raise ValueError(f'Expected exactly one native {name} module at its reviewed file')
        module = selected[0]
        expected = (ROOT / 'mixins/pluto_native_creative' / source_file).read_text()
        if (module.get('mod_id') != 'pluto_native_creative'
                or module.get('module_id') != 'pluto_native_creative-' + identifier
                or module.get('visibility') != 'pub' or module.get('source') != expected):
            raise ValueError(f'Native module identity, visibility or copied source changed: {name}')
    # The private-field seam is reviewed against the original implementation.
    # Additional item_data hooks/state changes need a separate combined review.
    for section in ('injections', 'states'):
        for change in plan.get(section, []):
            if (change.get('file') == meta['file']
                    or change.get('target', '').startswith('sunrise_items::item_data::')):
                raise ValueError('Additional item_data hooks/state invalidate the reviewed Wire seam')


if __name__ == '__main__':
    try:
        if len(sys.argv) != 3:
            raise ValueError('usage: check_wire_snapshot_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE')
        check(json.loads(Path(sys.argv[1]).read_text()), Path(sys.argv[2]))
    except (ValueError, OSError, KeyError) as error:
        print(f'Wire snapshot guard rejected: {error}', file=sys.stderr)
        raise SystemExit(1)
    print('Wire boundary verified; native preservation supplies no private reader or additive comparator.')
