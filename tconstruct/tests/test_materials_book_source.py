"""The initial guide uses pinned source text and the original acquisition recipe."""
import json
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ARCHIVE = ROOT / 'content'

class DirectoryArchive:
    def __init__(self, root): self.root = root
    def __enter__(self): return self
    def __exit__(self, *args): pass
    def read(self, name): return (self.root / name).read_bytes()
    def namelist(self): return [p.relative_to(self.root).as_posix() for p in self.root.rglob('*') if p.is_file()]
PREFIX = 'assets/tconstruct/book/materials_and_you/'

class MaterialsBookSourceTest(unittest.TestCase):
    def test_puny_worktable_and_further_reading_pages_are_available(self):
        root = ARCHIVE / 'assets/tconstruct/book/puny_smelting'
        index = json.loads((root / 'index.json').read_text())
        by_name = {chapter['name']: chapter['data'] for chapter in index}
        expected = {
            'worktable': ['index', 'modifier_worktable', 'invisible_ink', 'visible_ink',
                          'sort_modifiers', 'remove_modifiers', 'extract_modifier',
                          'extract_enchantment', 'extract_upgrade', 'extract_defense',
                          'extract_ability', 'toggle_interaction'],
            'further_reading': ['mighty_smelting', 'encyclopedia'],
        }
        for chapter, names in expected.items():
            entries = json.loads((root / by_name[chapter]).read_text())
            pages = [entry for entry in entries if entry.get('data') != 'no-load']
            self.assertEqual([entry['name'] for entry in pages], names)
            for entry in pages:
                page = json.loads((root / 'en_us' / entry['data']).read_text())
                self.assertTrue(page.get('title'))
                self.assertTrue(page.get('text') or page.get('description') or page.get('subText'))
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        self.assertIn('"casting" | "ranged_materials" | "armor" | "worktable" | "further_reading"', reader)

    def test_puny_ranged_material_chapter_uses_source_tier_and_predicate(self):
        root = ARCHIVE / 'assets/tconstruct/book/puny_smelting'
        index = json.loads((root / 'index.json').read_text())
        chapter = next(entry for entry in index if entry['name'] == 'ranged_materials')
        tier = chapter['extraData']['tconstruct:material_tier']
        self.assertEqual(tier['type'], 'tconstruct:ranged')
        self.assertEqual(tier['tier'], 2)
        self.assertEqual(tier['predicate'], {'type': 'mantle:inverted', 'inverted_type': 'mantle:tag', 'tag': 'tconstruct:book/distant'})
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        self.assertIn('show_material_chapter(puny, materials.len())', reader)
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-materials.zip') as archive:
            definition = json.loads(archive.read('data/tconstruct/tinkering/materials/definition/slimewood.json'))
            stats = json.loads(archive.read('data/tconstruct/tinkering/materials/stats/slimewood.json'))['stats']
            self.assertEqual(definition['tier'], 2)
            self.assertTrue({'tconstruct:limb', 'tconstruct:grip', 'tconstruct:bowstring'} & stats.keys())

    def test_puny_armor_pages_follow_source_tag_and_have_descriptions(self):
        root = ARCHIVE / 'assets/tconstruct/book/puny_smelting'
        index = json.loads((root / 'index.json').read_text())
        armor = next(entry for entry in index if entry['name'] == 'armor')
        tag = 'tconstruct:modifiable/book_armor/puny_smelting'
        self.assertEqual(armor['condition']['tag'], tag)
        self.assertEqual(armor['extraData']['tconstruct:load_tools']['tag'], tag)
        self.assertFalse(armor['extraData']['tconstruct:load_tools']['sort'])
        tag_file = ARCHIVE / 'reference/tconstruct-tags/data/tconstruct/tags/items/modifiable/book_armor/puny_smelting.json'
        ids = json.loads(tag_file.read_text())['values']
        self.assertEqual(len(ids), 10)
        for item in ids:
            page = json.loads((root / 'en_us/armor' / (item.replace(':', '_') + '.json')).read_text())
            self.assertEqual(page['tool'], item)
            self.assertTrue(page['text'])
            self.assertTrue(page['properties'])
        order = (ROOT / 'mixins/tconstruct/source-materials-book-index.rs').read_text()
        offsets = [order.index('"' + item + '"') for item in ids]
        self.assertEqual(offsets, sorted(offsets))

    def test_source_static_chapters_and_referenced_pages(self):
        with DirectoryArchive(ARCHIVE) as archive:
            index = json.loads(archive.read(PREFIX + 'index.json'))
            sections = [entry['data'] for entry in index if entry.get('data', 'no-load') != 'no-load']
            self.assertIn('sections/introduction.json', sections)
            self.assertIn('sections/tools.json', sections)
            self.assertIn('sections/modifiers.json', sections)
            readable = 0
            for section in sections:
                for entry in json.loads(archive.read(PREFIX + section)):
                    path = PREFIX + 'en_us/' + entry.get('data', '')
                    if path in archive.namelist():
                        page = json.loads(archive.read(path))
                        if page.get('text') or page.get('description'): readable += 1
            self.assertGreater(readable, 15)
            self.assertIn('assets/tconstruct/book/images/covers/materials_and_you_pages.png', archive.namelist())

    def test_book_recipe_uses_book_and_pattern(self):
        recipe = json.loads((ROOT / 'content/data/tconstruct/recipe/materials_and_you.json').read_text())
        self.assertEqual(set(recipe['ingredients']), {'minecraft:book', 'tconstruct:pattern'})
        self.assertEqual(recipe['result']['id'], 'tconstruct:materials_and_you')

    def test_starting_reward_uses_persisted_advancement(self):
        base = ROOT / 'content/data/tconstruct'
        advancement = json.loads((base / 'advancement/internal/starting_book.json').read_text())
        loot = json.loads((base / 'loot_table/gameplay/starting_book.json').read_text())
        self.assertEqual(advancement['criteria']['tick']['trigger'], 'minecraft:impossible')
        self.assertEqual(advancement['rewards']['loot'], ['tconstruct:gameplay/starting_book'])
        self.assertEqual(loot['pools'][0]['entries'][0]['name'], 'tconstruct:materials_and_you')

    def test_every_referenced_source_image_is_bundled(self):
        with DirectoryArchive(ARCHIVE) as archive:
            names = set(archive.namelist())
            for name in names:
                if '/en_us/' not in name or not name.endswith('.json'):
                    continue
                image = json.loads(archive.read(name)).get('image')
                if isinstance(image, dict) and image.get('file', '').startswith('tconstruct:book/images/'):
                    self.assertIn('assets/tconstruct/' + image['file'].split(':', 1)[1], names)

    def test_original_tool_and_armor_descriptions_are_retained(self):
        with DirectoryArchive(ARCHIVE) as archive:
            tool = json.loads(archive.read(PREFIX + 'en_us/tools/tconstruct_pickaxe.json'))
            armor = json.loads(archive.read('assets/tconstruct/book/encyclopedia/en_us/armor/tconstruct_travelers_helmet.json'))
            self.assertTrue(tool['text'])
            self.assertTrue(tool['properties'])
            self.assertTrue(armor['text'])
            self.assertTrue(armor['properties'])

    def test_modifier_showcases_have_source_slot_icons(self):
        with DirectoryArchive(ARCHIVE) as archive:
            names = set(archive.namelist())
            for page, slot, icon in [('abilities', 'abilities', 'ability'), ('defense', 'defense', 'defense'), ('slotless', 'slotless', 'slotless'), ('upgrades', 'upgrades', 'upgrade')]:
                data = json.loads(archive.read(PREFIX + 'en_us/modifiers/' + page + '.json'))
                self.assertEqual(data['item']['nbt']['slot'], slot)
                self.assertIn('assets/tconstruct/book/images/slot/' + icon + '.png', names)

    def test_material_panel_has_original_colors_and_part_stats(self):
        with DirectoryArchive(ARCHIVE) as archive:
            colors = json.loads(archive.read('assets/tconstruct/book/source-colors.json'))
            self.assertEqual(colors['material.tconstruct']['wood'], '#8e661b')
            self.assertEqual(colors['material.tconstruct']['flint'], '#696969')
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-materials.zip') as archive:
            wood = json.loads(archive.read('data/tconstruct/tinkering/materials/stats/wood.json'))['stats']
            self.assertEqual(wood['tconstruct:head']['durability'], 60)
            self.assertIn('tconstruct:handle', wood)
            self.assertIn('tconstruct:binding', wood)

    def test_tool_preview_uses_original_parts_and_table_art(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-tool-definitions.zip') as archive:
            data = json.loads(archive.read('data/tconstruct/tinkering/tool_definitions/pickaxe.json'))
            module = next(m for m in data['modules'] if m['type'] == 'tconstruct:part_stats')
            self.assertEqual(module['parts'], ['tconstruct:pick_head', 'tconstruct:tool_handle', 'tconstruct:tool_binding'])
        with DirectoryArchive(ARCHIVE) as archive:
            self.assertIn('assets/tconstruct/book/images/modify.png', archive.namelist())

    def test_original_repair_example_preserves_damage_values(self):
        with DirectoryArchive(ARCHIVE) as archive:
            repair = json.loads(archive.read(PREFIX + 'en_us/tools/repairing.json'))
            self.assertEqual(repair['grid'][0][0]['nbt']['Damage'], 70)
            self.assertEqual(repair['result']['nbt']['Damage'], 10)
            self.assertEqual(repair['result']['nbt']['tic_stats']['tconstruct:durability'], 100)

if __name__ == '__main__': unittest.main()
