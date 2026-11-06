"""Production interaction policy; wiring checks do not simulate Game/LAN."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import unittest
ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT/'mixins/tconstruct'

class MachineSneakPassthrough(unittest.TestCase):
    @unittest.skipUnless(shutil.which('rustc'), 'rustc not installed')
    def test_real_policy_for_all_hands_and_control_blocks(self):
        with tempfile.TemporaryDirectory() as tmp:
            harness = Path(tmp)/'lib.rs'
            harness.write_text(f'#[path="{MIXINS / "source-machine-interaction.rs"}"] mod policy;\n')
            binary = Path(tmp)/'tests'
            subprocess.run(['rustc','--edition=2021','--test',str(harness),'-o',str(binary)],check=True)
            subprocess.run([str(binary)],check=True)

    def test_policy_precedes_mutation_and_preserves_remote_delegation(self):
        text=(MIXINS/'source-machine-use.rs').read_text()
        guard=text.index('tconstruct_source_machine_interaction::sneak_passthrough(')
        self.assertLess(text.index('if game.remote.is_some() { return Continue(()); }'),guard)
        self.assertLess(text.index('let Some(hit) = hit else { return Continue(()); };'),guard)
        end=text.index('let pos =',guard)
        policy=text[guard:end]
        self.assertIn('game.keys.contains(&KeyCode::ShiftLeft)',policy)
        self.assertIn('game.stack_in(game.selected).is_some()',policy)
        self.assertIn('game.offhand.is_some()',policy)
        self.assertIn('return Continue(())',policy)
        for call in ['activate_faucet(', 'interact_casting(', 'game.swing_arm()', 'game.ensure_datapacks()']:
            self.assertLess(guard,text.index(call))
        manifest=tomllib.loads((MIXINS/'horizon-mixin.toml').read_text())
        module=next(m for m in manifest['module'] if m['id']=='source-machine-interaction')
        self.assertEqual(module['file'],'crates/horizon-client/src/game.rs')
        hook=next(m for m in manifest['injection'] if m['id']=='source-machine-use')
        self.assertEqual(hook['target'],"horizon_client::game::Game<'_>::use_hand")
        self.assertTrue(hook['cancellable'])

    @unittest.skipUnless(os.environ.get('HORIZON_HOST'), 'pinned host source not provided')
    def test_pinned_host_guest_context_supplies_guest_sneak_and_hands(self):
        host=Path(os.environ['HORIZON_HOST'])
        sha=subprocess.check_output(['git','-C',str(host),'rev-parse','HEAD'],text=True).strip()
        self.assertEqual(sha,'624c4f6265c02296fbe8a80f029dd0d987fb94c0')
        text=(host/'crates/horizon-client/src/game/lan_actions.rs').read_text()
        for line in ['std::mem::swap(&mut self.inventory, &mut g.inventory);',
                     'std::mem::swap(&mut self.offhand, &mut g.offhand);',
                     'let selected = std::mem::replace(&mut self.selected, g.selected);',
                     'let host_keys = std::mem::take(&mut self.keys);',
                     'if g.sneaking {\n            self.keys.insert(KeyCode::ShiftLeft);']:
            self.assertIn(line,text)
