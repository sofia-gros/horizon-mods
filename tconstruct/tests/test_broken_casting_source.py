import json
import unittest
from pathlib import Path
from zipfile import ZipFile
ROOT=Path(__file__).resolve().parents[1]
class BrokenCasting(unittest.TestCase):
    def test_fixture_uses_pinned_stock_helmet_materials_and_recipes(self):
        fixture=json.loads((ROOT/'tests/fixtures/broken_iron_helmet.json').read_text())
        for category,values in fixture.items():
            for key,value in values.items():
                path=key.split(':',1)[1]
                archive='tconstruct-recipes' if category=='recipes' else 'tconstruct-tool-definitions' if 'tool_definitions/' in path else 'tconstruct-materials'
                with ZipFile(ROOT/f'content/reference/{archive}.zip') as z:
                    prefix='recipes/' if category=='recipes' else ''
                    self.assertEqual(value,json.loads(z.read(f'data/tconstruct/{prefix}{path}.json')))
        with ZipFile(ROOT/'content/reference/tconstruct-modifiers.zip') as z:
            iron=json.loads(z.read('data/tconstruct/tinkering/modifiers/iron_armor.json'))
            projectile=json.loads(z.read('data/tconstruct/tinkering/modifiers/projectile_protection.json'))
        self.assertEqual([m['type'] for m in iron['modules']],['tconstruct:volatile_flag'])
        self.assertEqual([m['type'] for m in projectile['modules']],['tconstruct:max_armor_attribute','tconstruct:protection'])
    def test_casting_only_exception_and_damage_repair_order(self):
        world=(ROOT/'mixins/tconstruct/source-melter-world.rs').read_text()
        body=world.split('let was_broken =',1)[1].split('let supports_checked =',1)[0]
        self.assertLess(body.index('if composite && was_broken'),body.index('rebuild_casting_material_swap'))
        self.assertLess(body.index('fixed_set_damage(&mut output, repair_damage)'),body.index('casting_repair_amount('))
        self.assertIn('repair_damage.saturating_sub(repair)',body)
        self.assertIn('if output.damage >= max_damage { return None; }',body)
        station=(ROOT/'mixins/tconstruct/source-station-world.rs').read_text()
        self.assertIn('repair, reject_removed_traits, false, reason)',station)
        self.assertIn('identities, 0, true, true, reason)',station)
        self.assertIn('canonical.same_item_same_components(tool)',station)
        self.assertIn('allow_stock_helmet_repair && old_traits',station)
    def test_ci_runs_pinned_actual_candidate_and_assembly_fixture(self):
        ci=(ROOT.parent/'.github/workflows/mixins.yml').read_text()
        self.assertIn('tconstruct/tests/run_broken_casting_rust.sh',ci)
        runner=(ROOT/'tests/run_broken_casting_rust.sh').read_text()
        for text in ['pub(super) fn assemble(', 'pub(super) fn rebuild_casting_material_swap(', 'fn rebuild_material_swap_inner(', 'fixed_set_damage(', 'source-melter-world.rs','stock_broken_iron_helmet_casts_to_usable_steel_and_preserves_until_complete','0c7b5b61a5e746d621afffba919b12aeccf9c0fa','identity(p)']:
            self.assertIn(text,runner)
if __name__=='__main__':unittest.main()
