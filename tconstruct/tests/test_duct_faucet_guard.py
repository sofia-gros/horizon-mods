"""Routing regression: unsupported filters must never reach transfer fallbacks."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

class DuctFaucetGuard(unittest.TestCase):
    def test_production_policy(self):
        rustc = shutil.which('rustc')
        if rustc is None:
            self.skipTest('rustc unavailable')
        source = ROOT / 'mixins/tconstruct/source-machine-interaction.rs'
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'tests'
            # The production file is a child module (pub(super) visibility).
            harness = Path(directory) / 'lib.rs'
            harness.write_text(f'#[path = "{source}"] mod interaction;\n')
            subprocess.run([rustc, '--edition=2021', '--test', str(harness), '-o', str(target)], check=True)
            subprocess.run([str(target)], check=True)

    def test_dispatch_guard_precedes_all_transfer_routes(self):
        code = (ROOT / 'mixins/tconstruct/source-machine-runtime.rs').read_text()
        for start, end in [('pub(super) fn activate_faucet(', 'pub(super) fn tick_faucet('),
                           ('pub(super) fn tick_faucet(', 'pub(super) fn tick_casting(')]:
            body = code[code.index(start):code.index(end)]
            guard = body.index('tconstruct_source_machine_interaction::unsupported_duct(')
            for route in ('tank_to_cannon(', 'tank_to_casting_tank(', 'smeltery_to_tank(',
                          'tconstruct_source_faucet_tank::', 'smeltery_faucet('):
                self.assertLess(guard, body.index(route) if route != 'tconstruct_source_faucet_tank::' else
                                body.index(route, guard))
            self.assertIn('world.block(source.0, source.1, source.2)', body[guard:guard+200])
        source = code[code.index('fn smeltery_source('):code.index('fn selected_smeltery_fluid(')]
        self.assertNotIn('"tconstruct:seared_duct"', source)
        self.assertNotIn('"tconstruct:scorched_duct"', source)

if __name__ == '__main__':
    unittest.main()
