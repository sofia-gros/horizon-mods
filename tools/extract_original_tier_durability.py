#!/usr/bin/env python3
"""Generate 1.20.1 vanilla tier-tool durability facts from the pinned client.

Uses the JDK's javap module to read bytecode; it does not execute Minecraft.
Unrecognized items are deliberately omitted, not assigned guessed values.
"""
import argparse, hashlib, re, subprocess
from pathlib import Path
from source_data import CLIENT_SHA1


def javap(jar, cls):
    return subprocess.run(['java', '-m', 'jdk.jdeps/com.sun.tools.javap.Main',
                           '-classpath', str(jar), '-c', '-p', cls],
                          check=True, text=True, capture_output=True).stdout


def generate(jar):
    with jar.open('rb') as f:
        if hashlib.file_digest(f, 'sha1').hexdigest() != CLIENT_SHA1:
            raise ValueError('client jar does not match the pinned 1.20.1 hash')
    tiers = {}
    text = javap(jar, 'cho').split('  static {};', 1)[1]
    for block in re.split(r'(?m)^\s*\d+: new\s+#\d+\s+// class cho\s*$', text)[1:]:
        name = re.search(r'// String (WOOD|STONE|IRON|DIAMOND|GOLD|NETHERITE)', block)
        field = re.search(r'// Field ([a-f]):Lcho;', block)
        if not name or not field:
            continue
        instructions = re.findall(r'(?m)^\s*\d+: (iconst_m1|iconst_[0-5]|bipush\s+-?\d+|sipush\s+-?\d+)\s*$', block[name.end():])
        if len(instructions) < 3:
            raise ValueError('unsupported tier constructor bytecode')
        def number(op):
            return -1 if op == 'iconst_m1' else int(op.removeprefix('iconst_')) if op.startswith('iconst_') else int(op.split()[1])
        tiers[field.group(1)] = number(instructions[2])
    if len(tiers) != 6:
        raise ValueError('expected six original tool tiers')
    items = {}
    text = javap(jar, 'cgc')
    for block in re.split(r'(?m)^\s*\d+: ldc(?:_w)?\s+#\d+\s+// String ', text)[1:]:
        name = re.match(r'([a-z0-9_]+)\n', block)
        end = block.find('// Method a:(Ljava/lang/String;Lcfu;)Lcfu;')
        if not name or end < 0:
            continue
        tier = re.search(r'// Field cho\.([a-f]):Lcho;', block[:end])
        if tier:
            item = 'minecraft:' + name.group(1)
            if item in items:
                raise ValueError('duplicate original tier item')
            items[item] = tiers[tier.group(1)]
    if len(items) != 30:
        raise ValueError('expected 30 original tier tools')
    rows = ['// Generated from pinned Minecraft 1.20.1 bytecode by',
            '// tools/extract_original_tier_durability.py. Unknown items remain unknown.',
            'pub(super) fn max_damage(item: &str) -> Option<i32> {', '    match item {']
    rows.extend(f'        "{item}" => Some({value}),' for item, value in sorted(items.items()))
    rows.extend(['        _ => None,', '    }', '}', ''])
    return '\n'.join(rows)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('client_jar', type=Path)
    p.add_argument('output', type=Path)
    a = p.parse_args()
    a.output.write_text(generate(a.client_jar.resolve(strict=True)))

if __name__ == '__main__':
    main()
