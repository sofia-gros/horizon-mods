"""Run the pinned Java material cost method for the selected part recipes."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
from generate_numeric_oracle import block

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'third_party/tconstruct/src/main/java/slimeknights/tconstruct/library/recipe/material/IMaterialValue.java'
DATA = ROOT / 'third_party/tconstruct/src/generated/resources/data/tconstruct/recipes/tools/parts/builder'
TARGET = ROOT / 'tests/fixtures/part-cost-oracle.json'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    method = block(SOURCE.read_text(encoding='utf-8'), 'default int getItemsUsed(int itemCost)').replace('default int', 'static int', 1)
    java = '''public class PartCostOracle {
      static int getNeeded() { return 1; }
      static int getValue() { return 1; }
      %s
      public static void main(String[] args) {
        for (String arg : args) System.out.println(getItemsUsed(Integer.parseInt(arg)));
      }
    }''' % method
    names = ['pick_head', 'tool_handle', 'tool_binding']
    costs = [json.loads((DATA / f'{name}.json').read_text(encoding='utf-8'))['cost'] for name in names]
    with tempfile.TemporaryDirectory(prefix='part-cost-oracle-') as tmp:
        path = Path(tmp) / 'PartCostOracle.java'
        path.write_text(java, encoding='utf-8')
        output = subprocess.run([str(args.java.resolve()), str(path), *map(str, costs)],
                                capture_output=True, text=True, check=True, timeout=45)
    amounts = [int(line) for line in output.stdout.splitlines()]
    if len(amounts) != len(names):
        raise ValueError('unexpected oracle output')
    fixture = {'revision': 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2',
               'source_sha256': hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
               'scope': 'verbatim getItemsUsed with pinned cost and 1:1 wood/flint material values',
               'cases': [{'part': name, 'items': count} for name, count in zip(names, amounts)]}
    text = json.dumps(fixture, indent=2) + '\n'
    if args.check:
        if TARGET.read_text(encoding='utf-8') != text:
            raise ValueError('part cost oracle fixture changed')
    else:
        TARGET.write_text(text, encoding='utf-8')
    print(f'Java part cost oracle: {len(amounts)} cases verified')


if __name__ == '__main__':
    main()
