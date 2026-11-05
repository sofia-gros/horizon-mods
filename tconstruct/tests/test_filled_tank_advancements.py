"""Source filled-tank goals keep exact fluid amounts and stack/OR semantics."""
import copy
import hashlib
import json
from pathlib import Path
import re
import sys
import tempfile
import tomllib
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / 'content/data/tconstruct'
FIXTURES = ROOT / 'tests/fixtures/filled_tank'
GOALS = ('blaze', 'manyullyn_lanterns')
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class FilledTankAdvancements(unittest.TestCase):
    def test_exact_source_adaptation_and_hashes(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), {name + '.json' for name in GOALS})
        for name in GOALS:
            source = FIXTURES / (name + '.json')
            entry = manifest[source.name]
            self.assertEqual(hashlib.sha256(source.read_bytes()).hexdigest(), entry['sha256'])
            self.assertIn('/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/', entry['source'])
            expected = copy.deepcopy(json.loads(source.read_text()))
            expected['display']['icon'] = {'id': expected['display']['icon']['item']}
            for criterion in expected['criteria'].values():
                for item in criterion['conditions']['items']:
                    amount, fluid = re.fullmatch(r'\{tank:\{Amount:(\d+),FluidName:"([^"]+)"\}\}', item.pop('nbt')).groups()
                    item['type'] = 'tconstruct:portable_tank'
                    item['tank'] = {'fluid': fluid, 'amount': int(amount)}
            actual = json.loads((DATA / 'advancement/foundry' / source.name).read_text())
            self.assertEqual(actual, expected)
            self.assertEqual(actual['parent'], 'tconstruct:foundry/structure')
            self.assertNotIn('rewards', actual)

    def test_all_alternatives_remain_one_or_requirement(self):
        for name, count in [('blaze', 8), ('manyullyn_lanterns', 2)]:
            goal = json.loads((DATA / 'advancement/foundry' / (name + '.json')).read_text())
            self.assertEqual(len(goal['criteria']), count)
            self.assertEqual(len(goal['requirements']), 1)
            self.assertEqual(set(goal['requirements'][0]), set(goal['criteria']))
            for key, criterion in goal['criteria'].items():
                self.assertEqual(criterion['trigger'], 'minecraft:inventory_changed')
                self.assertEqual(len(criterion['conditions']['items']), 1)
                item = criterion['conditions']['items'][0]
                self.assertEqual(item['items'], ['tconstruct:' + key])
                if name == 'blaze':
                    self.assertNotIn('count', item)
                    self.assertEqual(item['tank'], {'fluid': 'tconstruct:blazing_blood',
                        'amount': 4320 if 'ingot' in key else 4000})
                else:
                    self.assertEqual(item['count'], {'min': 64})
                    self.assertEqual(item['tank'], {'fluid': 'tconstruct:molten_manyullyn', 'amount': 50})
                self.assertTrue((DATA / 'horizon/block' / (key + '.json')).is_file())

    def test_custom_predicate_is_cancellable_and_native_metadata_backed(self):
        mixins = tomllib.loads((ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text())
        hooks = [hook for hook in mixins['injection'] if hook['id'] == 'source-advancement-tank-predicate']
        self.assertEqual(len(hooks), 1)
        self.assertEqual(hooks[0]['target'], 'horizon_client::advancements::item')
        self.assertEqual(hooks[0]['kind'], 'HEAD')
        self.assertTrue(hooks[0]['cancellable'])
        helper = (ROOT / 'mixins/tconstruct' / hooks[0]['helper']).read_text()
        self.assertIn('custom.get("tconstruct:tank")?', helper)
        self.assertIn('actual_fluid.to_string().ok()?', helper)
        self.assertIn('Break(matches(v, it).unwrap_or(false))', helper)
        producer = (ROOT / 'mixins/tconstruct/source-melter-world.rs').read_text()
        self.assertIn('data.insert("tconstruct:tank".into(), tank.to_nbt())', producer)
        self.assertIn('catalog.portable_tank_filling', producer)

    def test_content_package_contains_both_goals_and_translations(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                language = json.loads(archive.read('assets/tconstruct/lang/en_us.json'))
                for name in GOALS:
                    packaged = 'data/tconstruct/advancement/foundry/' + name + '.json'
                    self.assertEqual(archive.read(packaged), (ROOT / 'content' / packaged).read_bytes())
                    for field in ('title', 'description'):
                        self.assertIn('advancements.tconstruct.foundry.' + name + '.' + field, language)


if __name__ == '__main__':
    unittest.main()
