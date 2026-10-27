"""Compare pickaxe base wear with the verbatim pinned Java getDamage method.

The minimal Java host models a primary harvest tool, not modifiers or Forge.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from generate_numeric_oracle import block

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'third_party/tconstruct/src/main/java/slimeknights/tconstruct/library/tools/helper/ToolHarvestLogic.java'
TARGET = ROOT / 'tests/fixtures/mining-wear-oracle.json'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    method = block(SOURCE.read_text(encoding='utf-8'), 'public static int getDamage(IToolStackView tool, Level world, BlockPos pos, BlockState state)')
    java = '''public class MiningWearOracle {
      static class Level {} static class BlockPos {}
      static class BlockState { float hardness; BlockState(float h) { hardness=h; }
        float getDestroySpeed(Level w, BlockPos p) { return hardness; }
        boolean is(Object t) { return false; } }
      interface IToolStackView { boolean hasTag(Object tag); }
      static class TinkerTags { static class Items { static final Object HARVEST=new Object(), HARVEST_PRIMARY=new Object(); } }
      static class BlockTags { static final Object FIRE=new Object(); }
      static class ToolActions { static final Object SHEARS_DIG=new Object(); }
      static class ModifierUtil { static boolean canPerformAction(IToolStackView t,Object a) { return false; } }
      %s
      public static void main(String[] a) {
        IToolStackView pickaxe = tag -> tag == TinkerTags.Items.HARVEST || tag == TinkerTags.Items.HARVEST_PRIMARY;
        for (float h : new float[] {0f, -0f, 1.5f, -1f})
          System.out.println(getDamage(pickaxe,new Level(),new BlockPos(),new BlockState(h)));
      }
    }''' % method
    with tempfile.TemporaryDirectory(prefix='mining-oracle-') as temp:
        file = Path(temp) / 'MiningWearOracle.java'
        file.write_text(java, encoding='utf-8')
        result = subprocess.run([str(args.java.resolve()), str(file)], capture_output=True, text=True, check=True, timeout=45)
    values = [int(line) for line in result.stdout.splitlines()]
    if len(values) != 4:
        raise ValueError('unexpected Java oracle output')
    fixture = {'revision': 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2',
               'source_sha256': hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
               'scope': 'verbatim getDamage for primary harvest pickaxe; no modifier or game runtime',
               'cases': [{'hardness': h, 'damage': d} for h, d in zip([0.0, -0.0, 1.5, -1.0], values)]}
    text = json.dumps(fixture, indent=2) + '\n'
    if args.check:
        if TARGET.read_text(encoding='utf-8') != text:
            raise ValueError('mining wear oracle fixture changed')
    else:
        TARGET.write_text(text, encoding='utf-8')
    print(f'Java mining wear oracle: {len(values)} cases verified')


if __name__ == '__main__':
    main()
