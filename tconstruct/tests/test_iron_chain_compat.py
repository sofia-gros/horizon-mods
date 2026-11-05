"""Native Iron Chain compatibility keeps pinned melting semantics."""
import json
from pathlib import Path
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'

class IronChainCompat(unittest.TestCase):
    def test_fixture_is_pinned_source_recipe(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            original = archive.read('data/tconstruct/recipes/smeltery/melting/metal/iron/chain.json')
        self.assertEqual(original, (ROOT / 'tests/fixtures/iron-chain-melting.json').read_bytes())
        value = json.loads(original)
        self.assertEqual(value, {'type': 'tconstruct:melting', 'ingredient': {'item': 'minecraft:chain'}, 'result': {'amount': 110, 'tag': 'forge:molten_iron'}, 'temperature': 800, 'time': 66})

    def test_both_preparation_paths_cache_and_world_override_order(self):
        manual = (MIXINS / 'prepare.rs').read_text()
        bundled = (MIXINS / 'resource-tables.rs').read_text()
        self.assertIn('tconstruct_source_chain_compat::prepare(&original_pack, &items)?', manual)
        self.assertIn('tconstruct_source_chain_compat::prepare(&original_out, &item_table)?', bundled)
        self.assertIn('tconstruct-native-creative-v8-scute-v1-chain-v1', bundled)
        self.assertLess(bundled.index('tconstruct_source_chain_compat::prepare'), bundled.rindex('std::fs::write(&marker'))
        self.assertIn('source = "source-chain-compat.rs"', (MIXINS / 'horizon-mixin.toml').read_text())
        reload = (MIXINS / 'recipe-reload.rs').read_text()
        self.assertLess(reload.index('merge(&original,'), reload.index('for id in &game.fns.enabled'))
