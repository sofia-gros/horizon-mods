"""Source descriptor and bounded Encyclopedia material-reader wiring."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
BOOK = ROOT / 'content/assets/tconstruct/book/encyclopedia'
CHAPTERS = {
    'materials_harvest': ('tconstruct:melee_harvest', 'melee_harvest_materials'),
    'materials_ranged': ('tconstruct:ranged', 'ranged_materials'),
    'materials_ammo': ('tconstruct:ammo', 'ammo_materials'),
    'materials_armor': ('tconstruct:armor', 'armor_materials'),
}


class EncyclopediaMaterialGuide(unittest.TestCase):
    def test_original_index_has_four_exact_single_type_descriptors(self):
        chapters = {entry['name']: entry for entry in json.loads((BOOK / 'index.json').read_text())}
        helper = (ROOT / 'mixins/tconstruct/source-materials-book-index.rs').read_text()
        for name, (kind, key) in CHAPTERS.items():
            with self.subTest(chapter=name):
                self.assertEqual(chapters[name]['data'], 'no-load')
                self.assertEqual(chapters[name]['extraData']['tconstruct:material_tier'],
                                 {'type': kind, 'detailed': True})
                self.assertIn(f'("{name}", "{kind}") => Some("{key}")', helper)
        self.assertIsInstance(chapters['materials_slime']['extraData']['tconstruct:material_tier'], list)
        self.assertIn('if data != "no-load" || !detailed { return None; }', helper)
        self.assertIn('(0..=i16::MAX as i32).contains(&tier)', helper)

    def test_reader_requires_descriptors_and_notices_before_rendering(self):
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        self.assertIn('.filter(|settings| settings.len() == 2)', reader)
        self.assertIn('settings.get("type")?.as_str()?, settings.get("detailed")?.as_bool()?', reader)
        self.assertIn('key != "intro" && encyclopedia_material.is_none()', reader)
        self.assertIn('labels.get(encyclopedia_heading.as_str()).copied()', reader)
        labels = dict(line.split('=', 1) for line in (BOOK / 'en_us/language.lang').read_text().splitlines()
                      if '=' in line and not line.startswith('#'))
        for name in CHAPTERS:
            self.assertIn('(Partial Guide)', labels[f'{name}.partial'])
        for key in ('materials.partial.notice_title', 'materials.partial.notice'):
            self.assertTrue(labels[key].strip())
            self.assertIn(f'labels.get("{key}").copied()', reader)
        self.assertIn('not proof that every effect or acquisition route works', labels['materials.partial.notice'])
        self.assertIn('Slimesuit materials', labels['intro.partial.notice'])
        self.assertNotIn('Only the introduction is available', labels['intro.partial.notice'])

    def test_full_tier_selection_and_all_renderer_paths_share_canonical_keys(self):
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        selection = reader[reader.index('fn dynamic_materials('):reader.index('fn recipe_graphic(')]
        self.assertIn('encyclopedia_material_tier(*tier)', selection)
        self.assertIn('!hidden && allowed', selection)
        self.assertIn('kinds.iter().any(|kind| s.contains_key(*kind))', selection)
        self.assertIn('let material_key = encyclopedia_material.unwrap_or(key);', reader)
        self.assertIn('dynamic_materials(game, material_key, puny, mighty, encyclopedia)', reader)
        self.assertIn('show_material_chapter(puny || mighty || encyclopedia, materials.len())', reader)
        self.assertIn('let visible: &[&str] = match material_key', reader)
        self.assertIn('let suffix = match material_key', reader)
        self.assertIn('material_sidebar(game, &material_context, &id, material_key, encyclopedia)', reader)
        self.assertIn('if material_key == "armor_materials"', reader)
        self.assertIn('groups.chunks(4)', reader)
        self.assertEqual(reader.count('=> super::tconstruct_materials_book_index::ENCYCLOPEDIA_ARMOR_STATS'), 3)
        self.assertIn('encyclopedia_armor_order(', selection)


if __name__ == '__main__':
    unittest.main()
