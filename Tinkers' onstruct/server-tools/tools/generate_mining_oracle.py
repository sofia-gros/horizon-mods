"""Run the pinned Java mining-speed methods in a minimal, non-Forge host."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = ROOT.parent / 'third_party/tconstruct/src/main/java/slimeknights/tconstruct/library/tools/definition/module/mining'
REVISION = 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2'


def method(source, marker):
    start = source.index(marker)
    opening = source.index('{', start)
    depth, end = 1, opening + 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return source[start:end]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    files = [UPSTREAM / 'MiningSpeedToolHook.java', UPSTREAM / 'IsEffectiveToolHook.java']
    sources = [file.read_text(encoding='utf-8') for file in files]
    methods = [method(sources[0], 'static float getDestroySpeed(IToolStackView tool, BlockState state)'),
               method(sources[1], 'static boolean isEffective(IToolStackView tool, BlockState state)')]
    cases = [dict(head=head, damage=damage, effective=effective, speed=speed)
             for head, damage, speed in [('wood', 0, 2.0), ('wood', 60, 2.0), ('flint', 17, 3.5), ('flint', 85, 3.5)]
             for effective in (False, True)]
    calls = []
    for case in cases:
        broken = case['damage'] == (60 if case['head'] == 'wood' else 85)
        calls.append('System.out.println(Integer.toUnsignedString(Float.floatToRawIntBits('
                     'MiningSpeedToolHook.getDestroySpeed(new Tool(%sf, %s), new BlockState(%s)))));'
                     % (case['speed'], str(broken).lower(), str(case['effective']).lower()))
    java = '''public class MiningOracle {
  record BlockState(boolean effective) {}
  static class Stats { final float value; Stats(float v) { value = v; } float get(int key) { return value; } }
  static class ToolStats { static final int MINING_SPEED = 1; }
  static class HookType<T> {}
  static class ToolHooks {
    static final HookType<MiningSpeedToolHook> MINING_SPEED = new HookType<>();
    static final HookType<IsEffectiveToolHook> IS_EFFECTIVE = new HookType<>();
  }
  interface IToolStackView { boolean isBroken(); Stats getStats(); <T> T getHook(HookType<T> hook); }
  interface MiningSpeedToolHook {
    float modifyDestroySpeed(IToolStackView tool, BlockState state, float speed);
    %s
  }
  interface IsEffectiveToolHook {
    boolean isToolEffective(IToolStackView tool, BlockState state);
    %s
  }
  static class Tool implements IToolStackView {
    final Stats stats; final boolean broken;
    Tool(float speed, boolean broken) { stats = new Stats(speed); this.broken = broken; }
    public boolean isBroken() { return broken; }
    public Stats getStats() { return stats; }
    @SuppressWarnings("unchecked") public <T> T getHook(HookType<T> hook) {
      Object value = hook == ToolHooks.MINING_SPEED
          ? (MiningSpeedToolHook) ((tool, state, speed) -> speed)
          : (IsEffectiveToolHook) ((tool, state) -> state.effective());
      return (T)value;
    }
  }
  public static void main(String[] args) { %s }
}
''' % (methods[0], methods[1], '\n'.join(calls))
    temporary = ROOT.parent.parent / '.local/tmp'
    temporary.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='mining-oracle-', dir=temporary) as folder:
        source = Path(folder) / 'MiningOracle.java'
        source.write_text(java, encoding='utf-8')
        javac = args.java.with_name('javac.exe' if args.java.suffix.lower() == '.exe' else 'javac')
        subprocess.run([str(javac), str(source)], check=True, capture_output=True)
        result = subprocess.run([str(args.java), '-cp', folder, 'MiningOracle'], check=True, capture_output=True, text=True)
    bits = [int(line) for line in result.stdout.splitlines()]
    if len(bits) != len(cases):
        raise ValueError('Java oracle returned an unexpected number of cases')
    for case, expected in zip(cases, bits):
        case['expected_bits'] = expected
    fixture = dict(schema=1, revision=REVISION,
                   scope='verbatim mining speed and effectiveness methods with identity modifiers; not full Forge gameplay',
                   source_sha256={file.name: hashlib.sha256(file.read_bytes()).hexdigest() for file in files},
                   extracted_methods_sha256=hashlib.sha256('\n'.join(methods).encode()).hexdigest(), cases=cases)
    target = ROOT / 'tests/fixtures/mining-speed-oracle.json'
    content = json.dumps(fixture, indent=2) + '\n'
    if args.check:
        if target.read_text(encoding='utf-8') != content:
            raise ValueError('Java mining speed oracle fixture changed')
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding='utf-8')
    print(f'Java mining oracle: {len(cases)} cases {"verified" if args.check else "generated"}')


if __name__ == '__main__':
    main()
