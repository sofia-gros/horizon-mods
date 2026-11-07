"""Run the real ItemStack projection and production injection with a small Game adapter.

This validates transactions without pretending the adapter is the full client.
The workflow separately applies and compiles the actual pinned client.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tomllib



def verify_source(horizon, expected_revision):
    revision = subprocess.check_output(['git', '-C', str(horizon), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != expected_revision:
        raise RuntimeError(f'Expected pinned Horizon {expected_revision}, got {revision}')
    subprocess.run(['git', '-C', str(horizon), 'diff', '--quiet', 'HEAD', '--'], check=True)


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
    target = root / 'target/gold-guard-fixture'
    target.mkdir(parents=True, exist_ok=True)
    dependencies = '\n'.join(
        f'{name} = {{path = {json.dumps(str(horizon / "crates" / name))}}}'
        for name in ('horizon-core', 'horizon-world', 'horizon-items', 'horizon-client-core'))
    (target / 'Cargo.toml').write_text(
        '[package]\nname="tconstruct-gold-guard-fixture"\nversion="0.1.0"\nedition="2024"\n'
        '[dependencies]\nserde_json="1"\n' + dependencies + '\n[lib]\npath="lib.rs"\n')
    shutil.copyfile(horizon / 'Cargo.lock', target / 'Cargo.lock')
    (target / 'lib.rs').write_text(
        f'#[path={json.dumps(str(root / "mixins/tconstruct/source-gold-guard.rs"))}]\nmod tconstruct_gold_guard;\n'
        + ''.join(f'include!({json.dumps(str(root / "tests/rust" / filename))});\n'
                  for filename in ('gold_guard_runtime.rs',)))
    subprocess.run(['cargo', 'metadata', '--offline', '--format-version', '1',
                    '--manifest-path', str(target / 'Cargo.toml')], stdout=subprocess.DEVNULL, check=True)
    verify_lock(horizon / 'Cargo.lock', target / 'Cargo.lock')
    result = subprocess.run(['cargo', 'test', '--locked', '--offline', '--manifest-path', str(target / 'Cargo.toml')],
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    print(result.stdout, end='')
    result.check_returncode()
    required = 'test game::live_projection_counts_native_and_tool_flags_and_does_not_stack ... ok'
    if required not in result.stdout:
        raise RuntimeError('Required production Gold Guard adapter test did not pass')
    if 'test game::revitalizing_same_refresh_gold_guard_swap_uses_final_maximum ... ok' not in result.stdout:
        raise RuntimeError('Required production Revitalizing adapter test did not pass')


if __name__ == '__main__':
    main()
