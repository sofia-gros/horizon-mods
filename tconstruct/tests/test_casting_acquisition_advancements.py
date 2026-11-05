"""Source casting goals, bounded tags, ancestry and packaged declarations."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
ADVANCEMENTS = ROOT / 'content/data/tconstruct/advancement'
FIXTURES = ROOT / 'tests/fixtures/casting_acquisition'
CASES = {
    'sand_casting': ('tconstruct:casts/single_use/blank', 'tconstruct:tool_handle_sand_cast'),
    'gold_casting': ('tconstruct:casts/gold', 'tconstruct:pick_head_cast'),
}
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class CastingAcquisitionAdvancements(unittest.TestCase):
    def test_pinned_source_criteria_display_and_requirements(self):
        manifest = json.loads((FIXTURES / 'manifest.json').read_text())
        self.assertEqual(set(manifest), set(CASES))
        bridge = (ROOT / 'mixins/tconstruct/source-advancement-tag.rs').read_text()
        for name, (tag, icon) in CASES.items():
            with self.subTest(advancement=name):
                raw = (FIXTURES / (name + '.json')).read_bytes()
                self.assertEqual(manifest[name], {'source': f'smeltery/{name}.json', 'sha256': hashlib.sha256(raw).hexdigest()})
                source = json.loads(raw)
                self.assertEqual(source['criteria']['crafted_cast']['conditions']['items'], [{'tag': tag}])
                expected = copy.deepcopy(source)
                expected['criteria']['crafted_cast']['conditions']['items'] = [{'items': '#' + tag}]
                expected['display']['icon'] = {'id': icon}
                native = json.loads((ADVANCEMENTS / f'smeltery/{name}.json').read_text())
                self.assertEqual(native, expected)
                self.assertEqual(native['parent'], 'tconstruct:smeltery/melter')
                self.assertEqual(native['requirements'], [['crafted_cast']])
                self.assertEqual(native['display']['frame'], 'task')
                self.assertTrue(native['display']['show_toast'])
                self.assertFalse(native['display']['announce_to_chat'])
                self.assertFalse(native['display']['hidden'])
                self.assertNotIn('rewards', native)
                self.assertIn('"' + tag + '"', bridge)

    def test_cast_collector_and_new_goals_have_complete_packaged_ancestry(self):
        for start in [*CASES, 'cast_collector']:
            name = 'smeltery/' + start
            seen = set()
            while name:
                self.assertNotIn(name, seen)
                seen.add(name)
                path = ADVANCEMENTS / (name + '.json')
                self.assertTrue(path.is_file(), name)
                parent = json.loads(path.read_text()).get('parent')
                if parent is None or not parent.startswith('tconstruct:'):
                    break
                name = parent.removeprefix('tconstruct:')
            if start == 'cast_collector':
                self.assertIn('smeltery/gold_casting', seen)
                self.assertIn('smeltery/melter', seen)

    def test_package_contains_goals_tags_members_and_icons(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for name, (tag, icon) in CASES.items():
                    goal = f'data/tconstruct/advancement/smeltery/{name}.json'
                    self.assertEqual(archive.read(goal), (ROOT / 'content' / goal).read_bytes())
                    tag_path = 'data/tconstruct/tags/items/' + tag.split(':', 1)[1] + '.json'
                    tag_data = json.loads((ROOT / 'content/reference/tconstruct-tags' / tag_path).read_text())
                    self.assertEqual(json.loads(archive.read('assets/tconstruct/source-packs/tconstruct-original-pack/' + tag_path)), tag_data)
                    for item in [icon, *tag_data['values']]:
                        namespace, item_name = item.split(':', 1)
                        self.assertIn(f'data/{namespace}/horizon/item/{item_name}.json', archive.namelist())


if __name__ == '__main__':
    unittest.main()
