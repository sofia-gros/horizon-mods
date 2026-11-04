"""Run the focused Melter-state fixture: pass the pinned Horizon checkout path."""
import pathlib, subprocess, sys, tempfile
root=pathlib.Path(__file__).resolve().parents[2]
if len(sys.argv)!=2: raise SystemExit(__doc__)
host=pathlib.Path(sys.argv[1]).resolve()
expected="0c7b5b61a5e746d621afffba919b12aeccf9c0fa"
try:
    actual=subprocess.check_output(['git','-C',str(host),'rev-parse','HEAD'],text=True).strip()
except subprocess.CalledProcessError as exc:
    raise SystemExit(f'Cannot read host revision: {exc}')
if actual != expected: raise SystemExit(f'Expected Horizon {expected}, got {actual}')
for crate in ('horizon-world','horizon-items'):
    if not (host/'crates'/crate/'Cargo.toml').is_file(): raise SystemExit(f'Missing {crate} under {host}')
def quote(path): return str(path).replace('\\','\\\\').replace('"','\\"')
with tempfile.TemporaryDirectory(prefix='melter-state-fixture-') as tmp:
    tmp=pathlib.Path(tmp); (tmp/'src').mkdir()
    (tmp/'Cargo.toml').write_text(f'''[package]
name="tconstruct-melter-state-fixture"
version="0.1.0"
edition="2021"
[dependencies]
horizon-world={{path="{quote(host/'crates/horizon-world')}"}}
horizon-items={{path="{quote(host/'crates/horizon-items')}"}}
''')
    (tmp/'src/lib.rs').write_text(f'#[path = "{quote(root/"tconstruct/tests/rust/melter_state_preservation.rs")}"] mod fixture;\n')
    subprocess.run(['cargo','test','--manifest-path',str(tmp/'Cargo.toml')],check=True)
