#!/usr/bin/env python3
"""Extract pinned Minecraft or Forge 1.20.1 item tags and recipes from a local jar."""
import argparse
import hashlib
import re
import zipfile
from pathlib import Path

# Mojang's 1.20.1 version manifest, downloads.client.sha1.
CLIENT_SHA1 = '0c3ec587af28e5a785c0b4a7b8a30f9a8f78f838'
FORGE_SHA256 = '2b0e97b22fd4d043dbbcc8e0a44ef0d58915689ed9224e4cd28d5c4976a16bcc'
PREFIX = 'data/minecraft/tags/items/'
FORGE_PREFIX = 'data/forge/tags/items/'
RECIPE_PREFIX = 'data/minecraft/recipes/'
VALID = re.compile(r'[a-z0-9_.-]+(?:/[a-z0-9_.-]+)*\.json\Z')


def extract(jar: Path, output: Path, expected_hash: str = CLIENT_SHA1, *, prefixes: tuple[str, ...] = (PREFIX, RECIPE_PREFIX), algorithm: str = "sha1") -> int:
    jar = jar.resolve(strict=True)
    output = output.absolute()
    if output.exists() or output.is_symlink():
        raise ValueError(f'output already exists: {output}')
    with jar.open('rb') as source:
        digest = hashlib.file_digest(source, algorithm).hexdigest()
    if digest != expected_hash:
        raise ValueError('jar does not match the pinned source hash')
    with zipfile.ZipFile(jar) as archive:
        files = []
        seen = set()
        for info in archive.infolist():
            prefix = next((p for p in prefixes if info.filename.startswith(p)), None)
            if prefix is None or info.is_dir():
                continue
            name = info.filename.removeprefix(prefix)
            if not VALID.fullmatch(name) or any(p in ('.', '..') for p in name.split('/')) or info.filename in seen:
                raise ValueError(f'invalid or duplicate tag path: {info.filename}')
            seen.add(info.filename)
            files.append((info.filename, archive.read(info)))
        if not files:
            raise ValueError('jar contains no requested data')
    for name, content in files:
        destination = output / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(content)
    return len(files)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--forge', action='store_true', help='extract Forge 1.20.1-47.4.0 universal jar tags')
    p.add_argument('client_jar', type=Path)
    p.add_argument('output', type=Path)
    args = p.parse_args()
    try:
        count = (extract(args.client_jar, args.output, FORGE_SHA256, prefixes=(FORGE_PREFIX, RECIPE_PREFIX), algorithm="sha256")
                 if args.forge else extract(args.client_jar, args.output))
    except (OSError, ValueError, zipfile.BadZipFile) as error:
        p.error(str(error))
    print(f'Extracted {count} original item tags and recipes into {args.output}')


if __name__ == '__main__':
    main()
