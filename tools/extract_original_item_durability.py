#!/usr/bin/env python3
"""Generate 1.20.1 vanilla item durability facts from the pinned client.

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

def number(op):
    return -1 if op == 'iconst_m1' else int(op.removeprefix('iconst_')) if op.startswith('iconst_') else int(op.split()[1])


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
        tiers[field.group(1)] = number(instructions[2])
    if len(tiers) != 6:
        raise ValueError('expected six original tool tiers')
    # Armor durability is the material multiplier times the slot base value.
    armor_text = javap(jar, 'cdl')
    base_section = armor_text.split('private static void a(java.util.EnumMap);', 1)[1].split('\n  private ', 1)[0]
    bases = {field: number(op) for field, op in re.findall(r'Field cdj\$a\.([a-d]):Lcdj\$a;\s*\n\s*\d+: (iconst_[0-5]|bipush\s+\d+|sipush\s+\d+)', base_section)}
    if len(bases) != 4:
        raise ValueError('expected four original armor slot durability values')
    armor_tiers = {}
    static = armor_text.split('  static {};', 1)[1]
    for block in re.split(r'(?m)^\s*\d+: new\s+#\d+\s+// class cdl\s*$', static)[1:]:
        multiplier = re.search(r'// String [a-z_]+\s*\n\s*\d+: (iconst_[0-5]|bipush\s+\d+|sipush\s+\d+)\s*\n\s*\d+: new', block)
        field = re.search(r'// Field ([a-g]):Lcdl;', block)
        if multiplier and field:
            armor_tiers[field.group(1)] = number(multiplier.group(1))
    if len(armor_tiers) != 7:
        raise ValueError('expected seven original armor materials')
    items = {}
    direct_items = 0
    text = javap(jar, 'cgc')
    for block in re.split(r'(?m)^\s*\d+: ldc(?:_w)?\s+#\d+\s+// String ', text)[1:]:
        name = re.match(r'([a-z0-9_]+)\n', block)
        end = block.find('// Method a:(Ljava/lang/String;Lcfu;)Lcfu;')
        if not name or end < 0:
            continue
        tier = re.search(r'// Field cho\.([a-f]):Lcho;', block[:end])
        direct = re.search(r'(?m)^\s*\d+: (iconst_[0-5]|bipush\s+\d+|sipush\s+\d+)\s*\n\s*\d+: invokevirtual\s+#\d+\s+// Method cfu\$a\.c:\(I\)Lcfu\$a;', block[:end])
        if tier and direct:
            raise ValueError('ambiguous durability construction')
        armor = re.search(r'// Field cdl\.([a-g]):Lcdl;.*?// Field cdj\$a\.([a-d]):Lcdj\$a;', block[:end], re.S)
        if armor and (tier or direct):
            raise ValueError('ambiguous armor construction')
        if tier or direct or armor:
            item = 'minecraft:' + name.group(1)
            if item in items:
                raise ValueError('duplicate original damageable item')
            items[item] = tiers[tier.group(1)] if tier else number(direct.group(1)) if direct else armor_tiers[armor.group(1)] * bases[armor.group(2)]
            direct_items += int(direct is not None)
    if len(items) != 66 or direct_items != 11:
        raise ValueError('expected 30 tier tools, 11 direct items and 25 armor items')
    rows = ['// Generated from pinned Minecraft 1.20.1 bytecode by',
            '// tools/extract_original_item_durability.py. Unknown items remain unknown.',
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
