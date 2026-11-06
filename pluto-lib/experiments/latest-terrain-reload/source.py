#!/usr/bin/env python3
"""Selected-source/lifecycle checks only. Never invokes Cargo, Rust or Mixins."""
import argparse
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
MARKER = b'    // INSERT EXACT NATIVE BASELINE\n'


def sha(body):
    return hashlib.sha256(body).hexdigest()


def read(root, path, digest):
    item = root / path
    if item.is_symlink() or not item.is_file():
        raise ValueError('Expected regular selected source: ' + path)
    data = item.read_bytes()
    if sha(data) != digest:
        raise ValueError('Selected source drift: ' + path)
    return data


def compose(fixture, extension):
    if fixture.count(MARKER) != 1 or extension in fixture:
        raise ValueError('Ambiguous fixture insertion')
    combined = fixture.replace(MARKER, extension + b'\n' + MARKER)
    if combined.replace(extension + b'\n' + MARKER, MARKER) != fixture:
        raise ValueError('Fixture inverse differs')
    return combined


def check(latest):
    value = json.loads((HERE / 'source_contract.json').read_text())
    current = {p: read(ROOT, p, h) for p, h in value['mod_sources'].items()}
    official = {p: read(latest, p, h).decode() for p, h in value['latest_sources'].items()}
    extension = read(HERE, 'native.rs', value['extension_sha256'])
    fixture = current[value['fixture']]
    combined = compose(fixture, extension)
    names = re.findall(rb'println!\("RELOAD_CASE (\w+) ok"\)', extension)
    if [n.decode() for n in names] != value['cases'] or len(set(names)) != 6:
        raise ValueError('Wrong six-case source inventory')
    if extension.count(b'#[test]') != 1:
        raise ValueError('Wrong unexecuted Rust test count')
    for item in value['ordered_spans']:
        text = official[item['path']]
        start = text.index(item['start'])
        end = text.index(item['end'], start + len(item['start']))
        part = text[start:end]
        at = -1
        for needle in item['ordered']:
            at = part.index(needle, at + 1)
    if any(value['execution'].values()):
        raise ValueError('Source candidate cannot claim native proof')
    return {'passed': True, 'scope': 'selected bytes, lexical call order and inverse composition only',
            'latest_files': len(official), 'mod_files': len(current), 'rust_tests_prepared': 1,
            'named_cases_prepared': len(names), 'composed_sha256': sha(combined),
            'native_executed': False, 'game_reload_executed': False, 'gpu_executed': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--latest-selected-source', type=Path, required=True)
    print(json.dumps(check(parser.parse_args().latest_selected_source), sort_keys=True))
