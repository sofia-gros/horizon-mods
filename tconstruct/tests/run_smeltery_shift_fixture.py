#!/usr/bin/env python3
"""Compile the production transfer function with a small host-ItemStack fixture.

Set HORIZON_ITEMS_RLIB and HORIZON_DEPS_DIR to matching compiled host artifacts.
This isolates the transfer, not the host menu, world store, or network.
"""
import os
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
source = (root / 'mixins/tconstruct/transaction.rs').read_text()
start = source.index('pub(super) fn source_give_inventory(')
end = source.index("/// A player's quick move first fills", start)
transfer = source[start:end]
fixture = (root / 'tests/rust/smeltery_shift_fixture.rs').read_text()
fixture = fixture.replace('// __PRODUCTION_TRANSFER__',
                          'mod extracted { use super::ItemStack;\n' + transfer +
                          '}\nuse extracted::source_give_inventory;')
with tempfile.TemporaryDirectory(prefix='smeltery-shift-') as directory:
    source = Path(directory) / 'fixture.rs'
    binary = Path(directory) / 'fixture'
    source.write_text(fixture)
    subprocess.run([os.environ.get('RUSTC', 'rustc'), '--edition=2021', '--test',
                    str(source), '-L', 'dependency=' + os.environ['HORIZON_DEPS_DIR'],
                    '--extern', 'horizon_items=' + os.environ['HORIZON_ITEMS_RLIB'],
                    '-o', str(binary)], check=True)
    subprocess.run([str(binary)], check=True)
