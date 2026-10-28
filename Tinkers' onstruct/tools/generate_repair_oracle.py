"""Run pinned upstream repair arithmetic in a minimal Java host.

The selected 1:1 material values and already-evaluated modifier factors are
supplied by the host. It is not a Forge inventory or modifier integration test.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
from generate_numeric_oracle import block

ROOT = Path(__file__).resolve().parents[1]
UP = ROOT / 'third_party/tconstruct/src/main/java/slimeknights/tconstruct'
MATERIAL = UP / 'library/recipe/material/MaterialRecipe.java'
REPAIR = UP / 'tables/recipe/TinkerStationRepairRecipe.java'
TARGET = ROOT / 'tests/fixtures/repair-oracle.json'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    scale = block(MATERIAL.read_text(encoding='utf-8'), 'public float scaleRepair(float amount)')
    repair = block(REPAIR.read_text(encoding='utf-8'), 'protected int repairFromSlot(ToolStack tool, ITinkerStationContainer inv, int repairNeeded, int slot, IntConsumer amountConsumer)')
    java = '''import java.util.function.IntConsumer;
    public class RepairOracle {
      static final float INGOTS_PER_REPAIR = 3f;
      int getValue() { return 1; } int getNeeded() { return 1; }
      %s
      static class ToolStack { final float factor; ToolStack(float f) { factor=f; } }
      static class ItemStack { final int count; ItemStack(int n) { count=n; }
        boolean isEmpty() { return count == 0; } int getCount() { return count; } }
      static class MaterialId { static final MaterialId UNKNOWN=new MaterialId(), KNOWN=new MaterialId(); }
      interface ITinkerStationContainer { ItemStack getInput(int slot); }
      static class RepairFactorModifierHook { static float getRepairFactor(ToolStack t, float n) { return n*t.factor; } }
      float head;
      MaterialId getMaterialFrom(ITinkerStationContainer inv, int slot) { return MaterialId.KNOWN; }
      float getRepairPerItem(ToolStack tool, ITinkerStationContainer inv, int slot, MaterialId material) { return scaleRepair(head); }
      %s
      public static void main(String[] args) {
        RepairOracle oracle = new RepairOracle();
        float[][] cases = {{85,2,85,1},{85,2,85,64},{60,2.5f,51,2},{60,2.5f,1,1}};
        for (float[] c : cases) {
          oracle.head=c[0]; int[] used={0};
          int result=oracle.repairFromSlot(new ToolStack(c[1]), slot -> new ItemStack((int)c[3]), (int)c[2], 0, n -> used[0]=n);
          System.out.println(used[0] + "," + result);
        }
      }
    }''' % (scale, repair)
    with tempfile.TemporaryDirectory(prefix='repair-oracle-') as tmp:
        path = Path(tmp) / 'RepairOracle.java'
        path.write_text(java, encoding='utf-8')
        result = subprocess.run([str(args.java.resolve()), str(path)], capture_output=True, text=True, check=True, timeout=45)
    outputs = [tuple(map(int, line.split(','))) for line in result.stdout.splitlines()]
    inputs = [('flint', 85, 1), ('flint', 85, 64), ('wood', 51, 2), ('wood', 1, 1)]
    if len(outputs) != len(inputs):
        raise ValueError('unexpected oracle output')
    fixture = {'revision': 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2',
               'source_sha256': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in (MATERIAL, REPAIR)},
               'scope': 'verbatim scaleRepair and repairFromSlot; 1:1 material, fixed factors; no Forge inventory',
               'cases': [{'head': head, 'damage': damage, 'available': available, 'items': items,
                          'repaired': min(damage, points)}
                         for (head, damage, available), (items, points) in zip(inputs, outputs)]}
    text = json.dumps(fixture, indent=2) + '\n'
    if args.check:
        if TARGET.read_text(encoding='utf-8') != text:
            raise ValueError('repair oracle fixture changed')
    else:
        TARGET.write_text(text, encoding='utf-8')
    print(f'Java repair oracle: {len(outputs)} cases verified')


if __name__ == '__main__':
    main()
