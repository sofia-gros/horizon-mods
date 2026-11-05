#!/usr/bin/env python3
"""Focused production Duct state/selection checks; not Game or LAN verification."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
root = Path(__file__).resolve().parents[2]
host = Path(os.environ['HORIZON_HOST']).resolve()
expected = json.loads((root/'tconstruct/compatibility.json').read_text())['horizon_revision']
actual = subprocess.check_output(['git','-C',str(host),'rev-parse','HEAD'], text=True).strip()
if actual != expected:
    raise SystemExit(f'Expected host {expected}, found {actual}')
with tempfile.TemporaryDirectory(prefix='duct-filter-') as directory:
    out = Path(directory)
    (out/'src').mkdir()
    modules = ''.join(f'#[path="{root}/tconstruct/mixins/tconstruct/source-{name}.rs"] mod tconstruct_source_{name.replace("-","_")};\n'
        for name in ['duct', 'smeltery-tank'])
    (out/'src/lib.rs').write_text(modules)
    (out/'Cargo.toml').write_text(f'''[package]
name="duct-filter-fixture"
version="0.1.0"
edition="2024"
[dependencies]
horizon-items={{path="{host}/crates/horizon-items"}}
horizon-world={{path="{host}/crates/horizon-world"}}
''')
    subprocess.run(['cargo','test','--manifest-path',str(out/'Cargo.toml'),'duct_'], check=True)
