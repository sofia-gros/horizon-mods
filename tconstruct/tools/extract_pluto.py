#!/usr/bin/env python3
"""Extract a PlutoLib .hnmod for staging with Horizon's official bundle reader."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

p = argparse.ArgumentParser()
p.add_argument('--horizon', type=Path, required=True)
p.add_argument('--package', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
horizon = a.horizon.resolve(strict=True)
source = Path(__file__).resolve().parent / 'native-extract/main.rs'
with tempfile.TemporaryDirectory(prefix='tconstruct-native-extract-') as temp:
    root = Path(temp)
    (root / 'src').mkdir()
    (root / 'src/main.rs').write_bytes(source.read_bytes())
    dep = json.dumps(str(horizon / 'crates/horizon-mods'))
    (root / 'Cargo.toml').write_text('[package]\nname = "tconstruct-native-extract"\nversion = "0.1.0"\nedition = "2024"\n[dependencies]\nhorizon-mods = { path = ' + dep + ', features = ["index"] }\n')
    subprocess.run(['cargo', 'run', '--quiet', '--manifest-path', str(root / 'Cargo.toml'), '--', str(a.package.resolve(strict=True)), str(a.output.resolve())], check=True, env=os.environ.copy())
