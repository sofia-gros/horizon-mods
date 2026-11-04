"""Run focused faucet fixtures against a local pinned Horizon checkout.

Usage: python3 tconstruct/tests/run_faucet_tank_fixture.py /path/to/horizon
Cargo must be configured with the checkout's dependencies; pass --offline to
Cargo through CARGO_NET_OFFLINE=true when the cache is available.
"""
import pathlib, subprocess, sys, tempfile

root = pathlib.Path(__file__).resolve().parents[2]
if len(sys.argv) != 2:
    raise SystemExit(__doc__)
host = pathlib.Path(sys.argv[1]).resolve()
for crate in ('horizon-world', 'horizon-items'):
    if not (host / 'crates' / crate / 'Cargo.toml').is_file():
        raise SystemExit(f'Missing {crate} under {host}')
with tempfile.TemporaryDirectory(prefix='faucet-tank-fixture-') as temp:
    temp = pathlib.Path(temp)
    (temp / 'src').mkdir()
    def rust_path(path):
        return str(path).replace('\\', '\\\\').replace('"', '\\"')
    manifest = f'''[package]
name = "tconstruct-faucet-tank-fixture"
version = "0.1.0"
edition = "2021"
[dependencies]
horizon-world = {{ path = "{rust_path(host / 'crates/horizon-world')}" }}
horizon-items = {{ path = "{rust_path(host / 'crates/horizon-items')}" }}
'''
    (temp / 'Cargo.toml').write_text(manifest)
    fixture = rust_path(root / 'tconstruct/tests/rust/faucet_tank_runtime.rs')
    state = rust_path(root / 'tconstruct/tests/rust/faucet_known_tanks.rs')
    (temp / 'src/lib.rs').write_text(f'''extern crate self as horizon_client;
pub mod containers {{ pub enum BlockEntity {{ Chest(Vec<()>), Other }} }}
#[path = "{fixture}"] mod runtime_fixture;
#[path = "{state}"] mod tank_fixture;
''')
    subprocess.run(['cargo', 'test', '--manifest-path', str(temp / 'Cargo.toml')], check=True)
