import json
from pathlib import Path
import tomllib
import unittest
ROOT = Path(__file__).resolve().parents[1]
class WitherBoneTests(unittest.TestCase):
    def test_source_modifier_contract(self):
        v = json.loads((ROOT / 'content/data/tconstruct/loot_modifiers/wither_bone.json').read_text())
        self.assertEqual(v, {'type':'mantle:replace_item','conditions':[{'condition':'forge:loot_table_id','loot_table_id':'minecraft:entities/wither_skeleton'},{'condition':'tconstruct:config','prop':'wither_bone_drop'}],'functions':[],'original':{'item':'minecraft:bone'},'replacement':'tconstruct:necrotic_bone'})
    def test_authoritative_hook_and_ci(self):
        v=tomllib.loads((ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text())
        hooks={x['id']:x for x in v['injection']}
        self.assertEqual(hooks['source-wither-bone-roll']['target'], 'horizon_client::mobs::drops::roll')
        self.assertEqual(hooks['source-wither-bone-roll']['kind'], 'RETURN')
        workflow=(ROOT.parent / '.github/workflows/mixins.yml').read_text()
        self.assertIn('tests/wither-bone/Cargo.toml',workflow)
        self.assertIn('tconstruct_wither_bone_host_tests::tests',workflow)
