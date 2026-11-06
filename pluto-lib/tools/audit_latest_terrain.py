#!/usr/bin/env python3
"""Read-only selected-source audit. Does not invoke Git, Cargo, Rust or Mixins."""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile


def digest(data):
    return hashlib.sha256(data).hexdigest()


def span(root, item):
    text = (root / item['path']).read_text()
    if text.count(item['start']) != 1:
        raise ValueError('Ambiguous span start: ' + item['path'])
    start = text.index(item['start'])
    end = text.index(item['end'], start)
    return text[start:end].encode()


def audit(args):
    report = json.loads(args.report.read_text())
    checks = []
    for group, old, new in [
        ('horizon_files', args.old_horizon, args.new_horizon),
        ('mixins_files', args.old_mixins, args.new_mixins),
    ]:
        rows = report[group]
        if len({r['path'] for r in rows}) != len(rows):
            raise ValueError('Duplicate source path')
        for row in rows:
            before, after = old / row['path'], new / row['path']
            if after.is_symlink() or not after.is_file():
                raise ValueError('New source must be a regular file')
            if digest(after.read_bytes()) != row['new_sha256'] or after.stat().st_size != row['new_bytes']:
                raise ValueError('New source drift: ' + row['path'])
            if row['old_sha256'] is None:
                if before.exists():
                    raise ValueError('Unexpected old source: ' + row['path'])
            elif before.is_symlink() or digest(before.read_bytes()) != row['old_sha256']:
                raise ValueError('Old source drift: ' + row['path'])
            if 'same' in row and row['same'] != (row['old_sha256'] == row['new_sha256']):
                raise ValueError('Incorrect equality classification')
        checks.append({'name': group, 'files': len(rows)})
    for item in report['exact_spans']:
        a, b = span(args.old_horizon, item), span(args.new_horizon, item)
        if a != b or digest(a) != item['sha256']:
            raise ValueError('Native span changed: ' + item['start'])
    checks.append({'name': 'exact_native_spans', 'count': len(report['exact_spans'])})
    new = {row['path']: row for row in report['horizon_files']}
    for group in report['guard_impact']:
        metadata = args.common_tools / group['metadata']
        if digest(metadata.read_bytes()) != group['metadata_sha256']:
            raise ValueError('Historical guard metadata changed')
        meta = json.loads(metadata.read_text())
        if meta['source_sha256'] != group['source_pins']:
            raise ValueError('Historical source pins changed')
        changed = [p for p, h in meta['source_sha256'].items() if new[p]['new_sha256'] != h]
        if changed != group['changed_inputs']:
            raise ValueError('Incorrect changed guard inputs')
    checks.append({'name': 'historical_guard_impact', 'groups': len(report['guard_impact'])})
    bundle = report['common_bundle']
    if args.common_bundle.stat().st_size != bundle['bytes'] or digest(args.common_bundle.read_bytes()) != bundle['sha256']:
        raise ValueError('Historical common archive changed')
    with zipfile.ZipFile(args.common_bundle) as archive:
        if len(archive.infolist()) != bundle['members']:
            raise ValueError('Historical common archive members changed')
    checks.append({'name': 'historical_common_archive', 'members': bundle['members']})
    if any(report['validation'].values()):
        raise ValueError('Source audit must not claim native execution or production changes')
    checks.append({'name': 'source_only_scope'})
    return {'passed': True, 'scope': 'selected source/hash comparison only', 'checks': checks,
            'native_plan': False, 'rust': False, 'cpu': False, 'gpu': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--report', type=Path, default=Path(__file__).resolve().parents[1] / 'docs/latest-official-terrain-audit-20261106.json')
    for name in ['old-horizon', 'new-horizon', 'old-mixins', 'new-mixins', 'common-tools', 'common-bundle']:
        parser.add_argument('--' + name, type=Path, required=True)
    print(json.dumps(audit(parser.parse_args()), sort_keys=True))
