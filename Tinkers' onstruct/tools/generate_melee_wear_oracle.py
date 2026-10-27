"""Compare pickaxe base melee wear against the pinned upstream Java snippet.

This small host evaluates the original successful-hit durability calculation;
it does not simulate Forge, modifier hooks, creative mode or attack eligibility.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'third_party/tconstruct/src/main/java/slimeknights/tconstruct/library/tools/helper/ToolAttackUtil.java'
TAGS = ROOT / 'third_party/tconstruct/src/generated/resources/data/tconstruct/tags/items/modifiable/melee'
TARGET = ROOT / 'tests/fixtures/melee-wear-oracle.json'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    source = SOURCE.read_text(encoding='utf-8')
    start = source.index('      int durabilityLost = targetLiving != null ? 1 : 0;')
    end = source.index('      // don\'t do the full damage animation', start)
    snippet = source[start:end]
    primary = json.loads((TAGS / 'primary.json').read_text(encoding='utf-8'))['values']
    weapon = json.loads((TAGS / 'weapon.json').read_text(encoding='utf-8'))['values']
    if 'tconstruct:pickaxe' not in weapon or 'tconstruct:pickaxe' in primary:
        raise ValueError('unexpected pinned pickaxe melee tags')
    java = '''public class MeleeWearOracle {
      interface Tool { boolean hasTag(Object tag); }
      static class TinkerTags { static class Items { static final Object MELEE_PRIMARY=new Object(); } }
      static int getDamage(Tool tool, Object targetLiving) {
      %s
      return durabilityLost;
      }
      public static void main(String[] args) {
        Tool pickaxe = tag -> false;
        System.out.println(getDamage(pickaxe, null));
        System.out.println(getDamage(pickaxe, new Object()));
      }
    }''' % snippet
    with tempfile.TemporaryDirectory(prefix='melee-oracle-') as temp:
        path = Path(temp) / 'MeleeWearOracle.java'
        path.write_text(java, encoding='utf-8')
        result = subprocess.run([str(args.java.resolve()), str(path)], capture_output=True, text=True, check=True, timeout=45)
    values = [int(line) for line in result.stdout.splitlines()]
    if len(values) != 2:
        raise ValueError('unexpected oracle output')
    fixture = {'revision': 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2',
               'source_sha256': hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
               'scope': 'verbatim successful-hit base durability snippet; pickaxe is not primary melee; no Forge or modifiers',
               'cases': [{'living_target': living, 'damage': damage}
                         for living, damage in zip([False, True], values)]}
    text = json.dumps(fixture, indent=2) + '\n'
    if args.check:
        if TARGET.read_text(encoding='utf-8') != text:
            raise ValueError('melee wear oracle fixture changed')
    else:
        TARGET.write_text(text, encoding='utf-8')
    print(f'Java melee wear oracle: {len(values)} cases verified')


if __name__ == '__main__':
    main()
