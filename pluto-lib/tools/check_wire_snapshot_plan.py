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


def check(plan, workspace, meta=None):
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
                    or change.get('target', '').startswith('horizon_items::item_data::')):
                raise ValueError('Additional item_data hooks/state invalidate the reviewed Wire seam')


if __name__ == '__main__':
    try:
        if len(sys.argv) != 3:
            raise ValueError('usage: check_wire_snapshot_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE')
        check(json.loads(Path(sys.argv[1]).read_text()), Path(sys.argv[2]))
    except (ValueError, OSError, KeyError) as error:
        print(f'Wire snapshot guard rejected: {error}', file=sys.stderr)
        raise SystemExit(1)
    print('Native Wire source, borrowed reader and additive comparator verified.')
