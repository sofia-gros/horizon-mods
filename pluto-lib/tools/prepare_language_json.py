#!/usr/bin/env python3
"""Prepare one flat TOML language file as JSON; never install it or replace output.

Runtime loading remains Horizon's JSON language loader. Dotted translation keys
must be quoted in TOML. Key spelling, Unicode and format strings stay unchanged.
"""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import tempfile
import tomllib

MAX_BYTES = 1 << 20
MAX_ENTRIES = 16384


def convert_toml(data: bytes, *, max_bytes: int = MAX_BYTES,
                 max_entries: int = MAX_ENTRIES) -> bytes:
    if max_bytes < 0 or max_entries < 0:
        raise ValueError('limits must be nonnegative')
    if len(data) > max_bytes:
        raise ValueError('input exceeds the byte limit')
    table = tomllib.loads(data.decode('utf-8'))
    if len(table) > max_entries:
        raise ValueError('input exceeds the entry limit')
    if any(not isinstance(value, str) for value in table.values()):
        raise ValueError('language TOML must be a flat string table; quote dotted keys')
    result = (json.dumps(table, ensure_ascii=False, sort_keys=True,
                         separators=(',', ':')) + '\n').encode('utf-8')
    if len(result) > max_bytes:
        raise ValueError('output exceeds the byte limit')
    return result


def prepare(source: Path, destination: Path) -> None:
    if source.suffix != '.toml' or destination.suffix != '.json':
        raise ValueError('expected a .toml source and a new .json output')
    # Bound the read before decoding/parsing; source stat alone is racy.
    with source.open('rb') as stream:
        data = stream.read(MAX_BYTES + 1)
    result = convert_toml(data)
    # Link an already complete sibling file into a previously absent name.
    # An existing file/symlink is never truncated. Unsupported hardlinks fail;
    # there is deliberately no overwrite/copy fallback.
    fd, name = tempfile.mkstemp(prefix='.pluto-language-', dir=destination.parent)
    temporary = Path(name)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(result)
        os.link(temporary, destination)
    finally:
        temporary.unlink()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('destination', type=Path)
    args = parser.parse_args()
    prepare(args.source, args.destination)


if __name__ == '__main__':
    main()
