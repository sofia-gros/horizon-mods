"""Run the real ItemStack projection and production injection with a small Game adapter.

This validates transactions without pretending the adapter is the full client.
The workflow separately applies and compiles the actual pinned client.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--horizon', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    horizon = args.horizon.resolve()
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
    subprocess.run(['cargo', 'test', '--offline', '--manifest-path', str(target / 'Cargo.toml')], check=True)


if __name__ == '__main__':
    main()
