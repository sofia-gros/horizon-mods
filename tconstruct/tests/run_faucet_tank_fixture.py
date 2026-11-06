"""Run focused faucet fixtures against a local pinned Horizon checkout.

Usage: python3 tconstruct/tests/run_faucet_tank_fixture.py /path/to/horizon
The checkout must match the pinned official revision. Its exact lockfile
dependencies must be cached; the fixture resolves and runs offline.
"""
from fixture_pluto_dependency import add_inventory_api, lock_fixture_to_host
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
    add_inventory_api(temp / 'Cargo.toml', root)
    fixture = rust_path(root / 'tconstruct/tests/rust/faucet_tank_runtime.rs')
    state = rust_path(root / 'tconstruct/tests/rust/faucet_known_tanks.rs')
    (temp / 'src/lib.rs').write_text(f'''include!("{rust_path(root / 'tconstruct/tests/rust/pluto_storage_fixture.rs')}");
pub mod containers {{ pub enum BlockEntity {{ Chest(Vec<()>), Other }} }}
#[path = "{fixture}"] mod runtime_fixture;
#[path = "{state}"] mod tank_fixture;
''')
    lock_fixture_to_host(temp / 'Cargo.toml', host, root)
    result = subprocess.run(['cargo', 'test', '--locked', '--offline', '--manifest-path', str(temp / 'Cargo.toml')],
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(result.stdout)
    result.check_returncode()
    for case in ('preserves_unknown_source_target_and_faucet_fields',
                 'preserves_unknown_melter_inner_tank_fields',
                 'stop_existing_faucet_preserves_faucet_and_pending_extensions',
                 'melter_tank_only_patch_preserves_opaque_machine_and_slots',
                 'lossy_reserved_packet_is_not_rewritten_when_stopping'):
        if f'test runtime_fixture::{case} ... ok' not in result.stdout.splitlines():
            raise SystemExit(f'Required storage preservation test did not run: {case}')
