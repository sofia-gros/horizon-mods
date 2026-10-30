#!/usr/bin/env python3
"""Extract vanilla 1.20.1 Item.Properties crafting remainders from bytecode."""
import argparse
import hashlib
import re
import subprocess
from pathlib import Path
from source_data import CLIENT_SHA1


def generate(jar: Path) -> str:
    with jar.open('rb') as stream:
        if hashlib.file_digest(stream, 'sha1').hexdigest() != CLIENT_SHA1:
            raise ValueError('client jar does not match the pinned 1.20.1 hash')
    text = subprocess.run(['java', '-m', 'jdk.jdeps/com.sun.tools.javap.Main',
                           '-classpath', str(jar), '-c', '-p', 'cgc'],
                          check=True, text=True, capture_output=True).stdout
    names = {}
    pairs = []
    for block in re.split(r'(?m)^\s*\d+: ldc(?:_w)?\s+#\d+\s+// String ', text)[1:]:
        name = re.match(r'([a-z0-9_]+)\n', block)
        end = block.find('// Method a:(Ljava/lang/String;Lcfu;)Lcfu;')
        if not name or end < 0:
            continue
        field = re.search(r'// Field ([A-Za-z0-9]+):Lcfu;', block[end:])
        if field:
            names[field.group(1)] = name.group(1)
        refs = re.findall(r'// Field ([A-Za-z0-9]+):Lcfu;\s*\n\s*\d+: invokevirtual\s+#\d+\s+// Method cfu\$a\.a:\(Lcfu;\)Lcfu\$a;', block[:end])
        if len(refs) > 1:
            raise ValueError('ambiguous crafting remainder')
        if refs:
            pairs.append((name.group(1), refs[0]))
    if len(pairs) != 5 or any(field not in names for _, field in pairs):
        raise ValueError('unsupported original crafting remainder bytecode')
    rows = ['// Generated from pinned Minecraft 1.20.1 bytecode by',
            '// tools/extract_original_crafting_remainders.py.',
            'pub(super) fn vanilla_remainder(item: &str) -> Option<&\'static str> {', '    match item {']
    rows += [f'        "minecraft:{item}" => Some("minecraft:{names[field]}"),' for item, field in sorted(pairs)]
    rows += ['        _ => None,', '    }', '}', '']
    return '\n'.join(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('client_jar', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    args.output.write_text(generate(args.client_jar.resolve(strict=True)))

if __name__ == '__main__':
    main()
