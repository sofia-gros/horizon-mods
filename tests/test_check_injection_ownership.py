import importlib.util
import json
import tempfile
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('check_injection_ownership', ROOT / 'tools/check_injection_ownership.py')
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


def manifest(directory, injections):
    folder = Path(directory)
    folder.mkdir(parents=True, exist_ok=True)
    text = '[mod]\nid = "x"\nversion = "0.1.0"\n'
    for ident, target in injections:
        text += f'\n[[injection]]\nid = "{ident}"\ntarget = "{target}"\nkind = "HEAD"\nhelper = "h.rs"\nbindings = []\n'
    path = folder / 'horizon-mixin.toml'
    path.write_text(text, encoding='utf-8', newline='\n')
    return path


class InjectionOwnership(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.base = Path(self.temp.name)

    def tearDown(self):
        self.temp.cleanup()

    def test_a_clean_layout_passes(self):
        pluto = manifest(self.base / 'pluto', [('a', 'game::one'), ('b', 'game::two')])
        mod = manifest(self.base / 'mod', [('c', 'game::three')])
        self.assertEqual(checker.problems(pluto, [mod], self.base / 'none.json'), [])

    def test_pluto_hooking_one_place_twice_is_reported(self):
        pluto = manifest(self.base / 'pluto', [('a', 'game::one'), ('b', 'game::one')])
        found = checker.problems(pluto, [], self.base / 'none.json')
        self.assertEqual(len(found), 1)
        self.assertIn('twice', found[0])

    def test_a_mod_hooking_a_place_pluto_owns_is_reported(self):
        pluto = manifest(self.base / 'pluto', [('a', 'game::one')])
        mod = manifest(self.base / 'mod', [('c', 'game::one')])
        found = checker.problems(pluto, [mod], self.base / 'none.json')
        self.assertEqual(len(found), 1)
        self.assertIn('owns', found[0])

    def test_a_recorded_overlap_is_tolerated_and_a_new_one_is_not(self):
        pluto = manifest(self.base / 'pluto', [('a', 'game::one')])
        mod = manifest(self.base / 'mod', [('c', 'game::one')])
        limits = self.base / 'limits.json'
        limits.write_text(json.dumps({'mod': {'known_overlaps': ['c']}}), encoding='utf-8', newline='\n')
        self.assertEqual(checker.problems(pluto, [mod], limits), [])
        limits.write_text(json.dumps({'mod': {'known_overlaps': ['other']}}), encoding='utf-8', newline='\n')
        self.assertEqual(len(checker.problems(pluto, [mod], limits)), 1)

    def test_a_mod_may_not_add_injections_beyond_the_recorded_number(self):
        pluto = manifest(self.base / 'pluto', [('a', 'game::one')])
        mod = manifest(self.base / 'mod', [('c', 'game::three'), ('d', 'game::four')])
        limits = self.base / 'limits.json'
        limits.write_text(json.dumps({'mod': {'max': 1}}), encoding='utf-8', newline='\n')
        found = checker.problems(pluto, [mod], limits)
        self.assertEqual(len(found), 1)
        self.assertIn('more than', found[0])


if __name__ == '__main__':
    unittest.main()
