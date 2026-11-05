"""Run the real ItemStack projection and production injection with a small Game adapter.

This validates the input projection without pretending the adapter is the full client.
The workflow separately applies and compiles the actual pinned client.
"""
import argparse
import json
import re
from pathlib import Path
import shutil
import subprocess
import tomllib



def verify_source(horizon, expected_revision):
    revision = subprocess.check_output(['git', '-C', str(horizon), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != expected_revision:
        raise RuntimeError(f'Expected pinned Horizon {expected_revision}, got {revision}')
    subprocess.run(['git', '-C', str(horizon), 'diff', '--quiet', 'HEAD', '--'], check=True)



def verify_native_body(horizon, helper):
    source = (horizon / 'crates/horizon-client/src/game.rs').read_text()
    start = source.index('    pub(crate) fn sync_player_body')
    start = source.index('{', start) + 1
    end = source.index('\n    }\n    fn held_slot', start)
    expected = re.sub(r'\bself\b', 'game', source[start:end])
    expected = expected.replace('let item_use_speed = item_in_use.map(|i| i.use_speed);', 'let item_use_speed = Some(speed);')
    expected = expected.replace('return;', 'return std::ops::ControlFlow::Break(());')
    actual = helper.read_text().split('// BEGIN PINNED NATIVE BODY', 1)[1].split('// END PINNED NATIVE BODY', 1)[0]
    if actual.split() != expected.split():
        raise RuntimeError('Use-speed compatibility body differs from the pinned native body')


def verify_lock(original, generated):
    # The fixture changes workspace membership, so its root/path packages differ.
    # Every fetched dependency must still be exactly one in the pinned host lock.
    def fetched(path):
        packages = tomllib.loads(path.read_text())['package']
        return {(p['name'], p['version'], p['source'], p.get('checksum'))
                for p in packages if 'source' in p}
    unexpected = fetched(generated) - fetched(original)
    if unexpected:
        raise RuntimeError(f'Fixture resolved dependencies outside pinned host lock: {sorted(unexpected)}')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--horizon', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    horizon = args.horizon.resolve()
    expected = json.loads((root / 'compatibility.json').read_text())['horizon_revision']
    verify_source(horizon, expected)
    verify_native_body(horizon, root / "mixins/tconstruct/use-speed-body.rs")
    target = root / 'target/use-speed-fixture'
    target.mkdir(parents=True, exist_ok=True)
    dependencies = '\n'.join(
        f'{name} = {{path = {json.dumps(str(horizon / "crates" / name))}}}'
        for name in ('horizon-core', 'horizon-world', 'horizon-items', 'horizon-client-core'))
    (target / 'Cargo.toml').write_text(
        '[package]\nname="tconstruct-use-speed-fixture"\nversion="0.1.0"\nedition="2024"\n'
        '[dependencies]\nserde_json="1"\n' + dependencies + '\n[lib]\npath="lib.rs"\n')
    shutil.copyfile(horizon / 'Cargo.lock', target / 'Cargo.lock')
    (target / 'lib.rs').write_text(
        ''.join(f'include!({json.dumps(str(root / "tests/rust" / filename))});\n'
                  for filename in ('use_speed_runtime.rs',)))
    subprocess.run(['cargo', 'metadata', '--offline', '--format-version', '1',
                    '--manifest-path', str(target / 'Cargo.toml')], stdout=subprocess.DEVNULL, check=True)
    verify_lock(horizon / 'Cargo.lock', target / 'Cargo.lock')
    result = subprocess.run(['cargo', 'test', '--locked', '--offline', '--manifest-path', str(target / 'Cargo.toml')],
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    print(result.stdout, end='')
    result.check_returncode()
    required = 'test game::actual_input_refresh_applies_held_and_worn_then_clears ... ok'
    if required not in result.stdout:
        raise RuntimeError('Required production Use Speed adapter test did not pass')


if __name__ == '__main__':
    main()
