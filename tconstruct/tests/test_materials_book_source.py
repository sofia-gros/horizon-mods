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
    def test_mighty_smelting_introduction_is_available_for_both_readers(self):
        root = ARCHIVE / 'assets/tconstruct/book/mighty_smelting'
        index = json.loads((root / 'index.json').read_text())
        intro = next(entry for entry in index if entry['name'] == 'intro')
        pages = json.loads((root / intro['data']).read_text())
        self.assertEqual([page['data'] for page in pages], ['intro/thruul.json', 'intro/tinkers_anvil.json', 'intro/tinkersanvil_craft.json'])
        for page in pages:
            data = json.loads((root / 'en_us' / page['data']).read_text())
            self.assertTrue(data.get('title'))
        self.assertTrue((root / 'en_us/language.lang').is_file())
        self.assertTrue((ARCHIVE / 'assets/tconstruct/book/images/covers/mighty_smelting_cover.png').is_file())
        self.assertTrue((ARCHIVE / 'assets/tconstruct/book/images/covers/mighty_smelting_pages.png').is_file())
        self.assertEqual(json.loads((ARCHIVE / 'data/tconstruct/horizon/item/mighty_smelting.json').read_text()), {'template': 'minecraft:book', 'max_stack_size': 1})
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            recipe = json.loads(archive.read('data/tconstruct/recipes/common/mighty_smelting.json'))
        self.assertEqual((recipe['result'], recipe['cast']['item'], recipe['cast_consumed'], recipe['fluid']['amount']), ('tconstruct:mighty_smelting', 'minecraft:book', True, 250))
        self.assertEqual((recipe['type'], recipe['cooling_time'], recipe['fluid']['tag']), ('tconstruct:casting_table', 89, 'tconstruct:seared_stone'))
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        request = (ROOT / 'mixins/tconstruct/source-materials-book-request.rs').read_text()
        self.assertIn('!matches!(key, "intro" | "tools" | "smeltery" | "melee_harvest_materials" | "ranged_materials" | "armor_materials")', reader)
        self.assertIn('mighty_advice_page', reader)
        self.assertIn('tools.partial.notice', reader)
        self.assertIn('source_pages(game, &path, book)', reader)
        self.assertIn('source_pages(game, &path, book)', request)

    def test_mighty_material_chapters_follow_source_tier_and_predicate(self):
        root = ARCHIVE / 'assets/tconstruct/book/mighty_smelting'
        index = json.loads((root / 'index.json').read_text())
        entries = {entry['name']: entry for entry in index}
        for name, kind in [('melee_harvest_materials', 'melee_harvest'),
                           ('ranged_materials', 'ranged'), ('armor_materials', 'armor')]:
            entry = entries[name]
            self.assertEqual(entry['data'], 'no-load')
            definition = entry['extraData']['tconstruct:material_tier']
            self.assertEqual((definition['type'], definition['tier']), (f'tconstruct:{kind}', 3))
            self.assertEqual(definition['predicate'], {'type': 'mantle:inverted',
                'inverted_type': 'mantle:tag', 'tag': 'tconstruct:book/distant'})
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        self.assertIn('mighty_material_tier(key, *tier, distant)', reader)
        self.assertIn('show_material_chapter(puny || mighty, materials.len())', reader)
        self.assertIn('Material entries reflect source data.', reader)

    def test_mighty_smeltery_basics_are_source_faithful_and_partial(self):
        import hashlib
        root = ARCHIVE / 'assets/tconstruct/book/mighty_smelting'
        for path, sha in {
            'sections/smeltery.json': '0125b0c787d061e86d1e74c12d157fa9bd3d2f32',
            'en_us/smeltery/controller.json': '68bd1e57a4e61f1a22136fef66127645f8d4a287',
            'en_us/smeltery/alloying.json': '49a047aca33345e2759712225b8741233df68733',
        }.items():
            data = (root / path).read_bytes()
            self.assertEqual(hashlib.sha1(f'blob {len(data)}\0'.encode() + data).hexdigest(), sha)
        image = (ARCHIVE / 'assets/tconstruct/book/images/scontroller.png').read_bytes()
        self.assertEqual(hashlib.sha1(f'blob {len(image)}\0'.encode() + image).hexdigest(), 'f6876cde9a983114a3208992282474f4dc9d0d23')
        section = json.loads((root / 'sections/smeltery.json').read_text())
        self.assertEqual([(page['name'], page['data']) for page in section if page['name'] in ('controller', 'alloying')],
                         [('controller', 'smeltery/controller.json'), ('alloying', 'smeltery/alloying.json')])
        language = (root / 'en_us/language.lang').read_text()
        self.assertIn('smeltery.partial=Smeltery Basics (Partial)', language)
        self.assertIn('Entity melting and other machine behavior are incomplete.', language)
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        self.assertIn('mighty_smeltery_page', reader)
        self.assertIn('smeltery.partial.notice', reader)
        self.assertIn('mighty_advice || mighty_smeltery', reader)

    def test_mighty_tool_advice_is_source_faithful_and_explicitly_partial(self):
        import hashlib
        root = ARCHIVE / 'assets/tconstruct/book/mighty_smelting'
        expected = {
            'sections/tools.json': '793d15d696add96370727a247b4d52d566b0ec59',
            'en_us/tools/tips.json': 'fc04b018d35a689015f34d045dfdfef095d45fdd',
            'en_us/tools/designing.json': '6c0ada56181a2083df5729814193858d1a392cda',
        }
        for path, sha in expected.items():
            data = (root / path).read_bytes()
            self.assertEqual(hashlib.sha1(f'blob {len(data)}\0'.encode() + data).hexdigest(), sha)
        section = json.loads((root / 'sections/tools.json').read_text())
        self.assertEqual([page['name'] for page in section], ['group_general', 'group_ranged', 'group_info', 'tanks', 'tips', 'designing'])
        language = (root / 'en_us/language.lang').read_text()
        self.assertIn('tools.partial=Tool Advice (Partial)', language)
        self.assertIn('material tip icon mentioned here is not available', language)
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        self.assertIn('mighty_advice_page', reader)
        self.assertIn('chapter_pages.splice(0..0, notices)', reader)

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
        self.assertIn('"casting" | "melee_harvest_materials" | "ranged_materials" | "armor_materials" | "armor" | "upgrades" | "slotless" | "worktable" | "further_reading"', reader)

    def test_puny_upgrade_static_overviews_are_available(self):
        root = ARCHIVE / 'assets/tconstruct/book/puny_smelting'
        index = json.loads((root / 'index.json').read_text())
        chapter = next(entry for entry in index if entry['name'] == 'upgrades')
        entries = json.loads((root / chapter['data']).read_text())
        pages = [page for page in entries if 'data' in page]
        self.assertEqual([page['data'] for page in pages], [
            'upgrades/general.json', 'upgrades/melee.json', 'upgrades/harvest.json',
            'upgrades/ranged.json', 'upgrades/armor.json'])
        for page in pages:
            content = json.loads((root / 'en_us' / page['data']).read_text())
            self.assertTrue(content.get('title'))
            self.assertTrue(content.get('text'))
        self.assertEqual(sum('extraData' in page for page in entries), 10)

    def test_puny_slotless_static_overviews_are_available(self):
        root = ARCHIVE / 'assets/tconstruct/book/puny_smelting'
        index = json.loads((root / 'index.json').read_text())
        chapter = next(entry for entry in index if entry['name'] == 'slotless')
        pages = json.loads((root / chapter['data']).read_text())
        self.assertEqual([page['data'] for page in pages], [
            'slotless/slotless.json', 'slotless/damage_tools.json',
            'slotless/bonus_upgrades.json', 'slotless/cosmetic.json'])
        for page in pages:
            content = json.loads((root / 'en_us' / page['data']).read_text())
            self.assertTrue(content.get('title'))
            self.assertTrue(content.get('text'))
        self.assertEqual(sum('extraData' in page for page in pages), 3)

    def test_puny_melee_material_predicate_matches_source_index(self):
        root = ARCHIVE / 'assets/tconstruct/book/puny_smelting'
        index = json.loads((root / 'index.json').read_text())
        chapter = next(entry for entry in index if entry['name'] == 'melee_harvest_materials')
        tier = chapter['extraData']['tconstruct:material_tier']
        self.assertEqual((tier['type'], tier['tier']), ('tconstruct:melee_harvest', 2))
        predicate = tier['predicate']
        self.assertEqual(predicate['type'], 'mantle:or')
        self.assertEqual(predicate['predicates'][0], {'type': 'mantle:inverted', 'inverted_type': 'mantle:tag', 'tag': 'tconstruct:book/distant'})
        whitestone = predicate['predicates'][1]
        self.assertEqual(whitestone['conditions'], [{'type': 'mantle:tag_filled', 'tag': 'tconstruct:whitestone_ingots'}])
        self.assertEqual(whitestone['if_true'], {'type': 'tconstruct:id', 'id': 'tconstruct:whitestone'})
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        self.assertIn('puny_melee_material_tier(*tier, distant, id, whitestone_ingots)', reader)

    def test_puny_ranged_material_chapter_uses_source_tier_and_predicate(self):
        root = ARCHIVE / 'assets/tconstruct/book/puny_smelting'
        index = json.loads((root / 'index.json').read_text())
        chapter = next(entry for entry in index if entry['name'] == 'ranged_materials')
        tier = chapter['extraData']['tconstruct:material_tier']
        self.assertEqual(tier['type'], 'tconstruct:ranged')
        self.assertEqual(tier['tier'], 2)
        self.assertEqual(tier['predicate'], {'type': 'mantle:inverted', 'inverted_type': 'mantle:tag', 'tag': 'tconstruct:book/distant'})
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        self.assertIn('show_material_chapter(puny || mighty, materials.len())', reader)
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-materials.zip') as archive:
            definition = json.loads(archive.read('data/tconstruct/tinkering/materials/definition/slimewood.json'))
            stats = json.loads(archive.read('data/tconstruct/tinkering/materials/stats/slimewood.json'))['stats']
            self.assertEqual(definition['tier'], 2)
            self.assertTrue({'tconstruct:limb', 'tconstruct:grip', 'tconstruct:bowstring'} & stats.keys())

    def test_puny_armor_material_chapter_uses_source_range_and_stats(self):
        root = ARCHIVE / 'assets/tconstruct/book/puny_smelting'
        chapter = next(entry for entry in json.loads((root / 'index.json').read_text()) if entry['name'] == 'armor_materials')
        tier = chapter['extraData']['tconstruct:material_tier']
        self.assertEqual(tier['type'], 'tconstruct:armor')
        self.assertEqual(tier['tier'], {'min': 0, 'max': 2})
        self.assertEqual(tier['predicate'], {'type': 'mantle:inverted', 'inverted_type': 'mantle:tag', 'tag': 'tconstruct:book/distant'})
        reader = (ROOT / 'mixins/tconstruct/source-materials-book.rs').read_text()
        for stat in ('plating_helmet', 'plating_chestplate', 'plating_leggings', 'plating_boots', 'plating_shield', 'maille', 'shield_core', 'cuirass', 'slime', 'skull', 'shell', 'laces', 'ribcage'):
            self.assertIn('"tconstruct:' + stat + '"', reader)
        self.assertIn('groups.chunks(4)', reader)
        self.assertIn('show_material_chapter(puny || mighty, materials.len())', reader)
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-materials.zip') as archive:
            stats = json.loads(archive.read('data/tconstruct/tinkering/materials/stats/iron.json'))['stats']
            self.assertIn('tconstruct:plating_helmet', stats)
            self.assertIn('tconstruct:maille', stats)

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
