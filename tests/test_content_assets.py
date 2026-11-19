"""The imported client assets (tools/import_original_data.py) must resolve inside the content mod and the pinned base assets."""
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'tconstruct/content'
ASSETS = CONTENT / 'assets/tconstruct'
DECLARED = CONTENT / 'data/tconstruct/horizon'
MATERIALS = CONTENT / 'data/tconstruct/tinkering/materials/definition'
TOOLS = CONTENT / 'data/tconstruct/tinkering/tool_definitions'
PART_TABLE = ROOT / 'tconstruct/mixins/tconstruct/api/src/tool_parts.rs'
ARMOR_TABLE = ROOT / 'tconstruct/mixins/tconstruct/api/src/armor_textures.rs'
BASE = ROOT / '.local/base-26.3/assets/minecraft'  # a read-only reference checkout; absent on some runners
# Declared items whose original model is plain (no tool/material loader); the rest wait for per-material rendering.
SIMPLE_ITEMS = {'modifier_crystal', 'pattern', 'venom_bottle'}


def load(path):
    return json.loads(path.read_text(encoding='utf-8'))


def split(reference):
    namespace, _, path = reference.partition(':') if ':' in reference else ('minecraft', '', reference)
    return namespace, path


class ContentAssets(unittest.TestCase):
    def resolves(self, kind, extension, reference):
        namespace, path = split(reference)
        if namespace == 'tconstruct':
            return (ASSETS / kind / f'{path}.{extension}').is_file()
        return namespace == 'minecraft' and (not BASE.is_dir() or (BASE / kind / f'{path}.{extension}').is_file())

    def test_every_model_parent_and_texture_resolves(self):
        models = sorted((ASSETS / 'models').rglob('*.json'))
        self.assertTrue(models)
        parents = {split(load(p)['parent'])[1] for p in models if 'parent' in load(p)}
        for path in models:
            model = load(path)
            leaf = path.relative_to(ASSETS / 'models').with_suffix('').as_posix() not in parents
            self.assertNotIn('loader', model, path)
            if 'parent' in model:
                self.assertTrue(self.resolves('models', 'json', model['parent']), f'{path}: parent {model["parent"]}')
            for name, texture in model.get('textures', {}).items():
                if not texture.startswith('#'):
                    self.assertTrue(self.resolves('textures', 'png', texture), f'{path}: texture {name}={texture}')
            for element in model.get('elements', []):
                for face in element['faces'].values():
                    reference = face['texture']
                    if leaf and reference.startswith('#'):
                        self.assertIn(reference[1:], self.inherited_textures(model), f'{path}: unbound {reference}')

    def inherited_textures(self, model):
        names = dict(model.get('textures', {}))
        parent = model.get('parent')
        while parent and split(parent)[0] == 'tconstruct':
            model = load(ASSETS / 'models' / f'{split(parent)[1]}.json')
            names = {**model.get('textures', {}), **names}
            parent = model.get('parent')
        return names

    def test_declared_blocks_and_simple_items_have_definitions(self):
        blocks = {p.stem for p in (DECLARED / 'block').glob('*.json')}
        items = {p.stem for p in (DECLARED / 'item').glob('*.json')}
        self.assertTrue(SIMPLE_ITEMS <= items)
        for name in blocks | SIMPLE_ITEMS:
            definition = load(ASSETS / 'items' / f'{name}.json')
            self.assertEqual(definition['model']['type'], 'minecraft:model')
            self.assertTrue(self.resolves('models', 'json', definition['model']['model']), name)
        for name in blocks:
            for variant in load(ASSETS / 'blockstates' / f'{name}.json')['variants'].values():
                self.assertTrue(self.resolves('models', 'json', variant['model']), name)
        for path in (ASSETS / 'items').glob('*.json'):
            self.assertIn(path.stem, blocks | items, f'item definition for an undeclared id: {path.name}')
        for path in (ASSETS / 'blockstates').glob('*.json'):
            self.assertIn(path.stem, blocks, f'blockstate for an undeclared block: {path.name}')

    def test_every_registered_part_selects_its_material_sprite(self):
        """tool_parts.rs registers the parts (plus the repair kit): each one's definition is a select on the first
        custom_model_data string, whose cases are shipped material ids and whose every model resolves."""
        parts = {item for item, _ in re.findall(r'\("([a-z_]+)", "([a-z_]+)"\)', PART_TABLE.read_text(encoding='utf-8').split('const PARTS')[1].split('];')[0])}
        parts.add('repair_kit')
        self.assertEqual(len(parts), 28)
        materials = {p.stem for p in MATERIALS.glob('*.json')}
        for part in sorted(parts):
            model = load(ASSETS / 'items' / f'{part}.json')['model']
            self.assertEqual((model['type'], model['property'], model['index']), ('minecraft:select', 'minecraft:custom_model_data', 0), part)
            seen = set()
            for case in model['cases']:
                self.assertIn(case['when'], materials, part)
                self.assertNotIn(case['when'], seen, part)
                seen.add(case['when'])
                self.assertEqual(case['model']['type'], 'minecraft:model')
                self.assertTrue(self.resolves('models', 'json', case['model']['model']), f'{part}: {case["when"]}')
            self.assertTrue(self.resolves('models', 'json', model['fallback']['model']), part)
            self.assertTrue(seen, part)

    def test_every_layered_tool_composes_material_layers_by_slot(self):
        """Each tool, armor piece and shield is one plain model (staffs, flint and brick) or a composite of selects whose index is
        a material slot of its tool definition and whose cases are shipped material ids with resolving models."""
        materials = {p.stem for p in MATERIALS.glob('*.json')}
        tools = sorted(p.stem for p in TOOLS.glob('*.json'))
        self.assertEqual(len(tools), 44)
        for tool in tools:
            definition = load(TOOLS / f'{tool}.json')
            slots = sum(len(m.get('parts') or m.get('stat_types') or []) for m in definition['modules'] if m['type'] in ('tconstruct:part_stats', 'tconstruct:material_stats'))
            model = load(ASSETS / 'items' / f'{tool}.json')['model']
            if slots == 0:
                self.assertEqual(model['type'], 'minecraft:model', tool)
                self.assertTrue(self.resolves('models', 'json', model['model']), tool)
                continue
            self.assertEqual(model['type'], 'minecraft:composite', tool)
            self.assertTrue(model['models'], tool)
            for layer in model['models']:
                self.assertEqual((layer['type'], layer['property']), ('minecraft:select', 'minecraft:custom_model_data'), tool)
                self.assertLess(layer['index'], slots, tool)
                self.assertTrue(self.resolves('models', 'json', layer['fallback']['model']), tool)
                seen = set()
                for case in layer['cases']:
                    self.assertIn(case['when'], materials, tool)
                    self.assertNotIn(case['when'], seen, tool)
                    seen.add(case['when'])
                    self.assertTrue(self.resolves('models', 'json', case['model']['model']), f'{tool}: {case["when"]}')

    def test_every_texture_is_used(self):
        used = set()
        for path in (ASSETS / 'models').rglob('*.json'):
            used |= {split(t)[1] for t in load(path).get('textures', {}).values() if split(t)[0] == 'tconstruct'}
        present = {p.relative_to(ASSETS / 'textures').with_suffix('').as_posix() for p in (ASSETS / 'textures').rglob('*.png')
                   if not p.relative_to(ASSETS / 'textures').as_posix().startswith('entity/equipment/')}
        self.assertEqual(present, used)

    def test_worn_armor_textures_are_exactly_the_ones_the_layer_table_can_pick(self):
        """armor_textures.rs names every worn texture: each base and each material variant is a shipped PNG in the folder of its
        layer set, nothing else is shipped there, and every equipment asset and armor declaration resolves."""
        text = ARMOR_TABLE.read_text(encoding='utf-8')
        bases = re.findall(r'^    "([a-z_/]+)",$', text.split('const BASES')[1].split('];')[0], re.M)
        variants = re.findall(r'^    \("([a-z_/]+)", "([a-z_]+)", "([a-z_]*)", (?:None|Some\(0x[0-9A-F]{6}\))\),$', text.split('const VARIANTS')[1], re.M)
        self.assertTrue(bases and variants)
        folders = {'armor': 'humanoid', 'leggings': 'humanoid_leggings', 'wings': 'wings'}

        def folder(stem):
            name = stem.rsplit('/', 1)[1]
            return folders[next(word for word in folders if re.search(rf'(^|_){word}(_|$)', name))]

        stems = set(bases) | {base + suffix for base, _, suffix in variants}
        materials = {p.stem for p in MATERIALS.glob('*.json')} | {'unknown'}
        for base, material, suffix in variants:
            self.assertIn(base, bases)
            self.assertIn(material, materials, base)
        shipped = {p.relative_to(ASSETS / 'textures/entity/equipment').with_suffix('').as_posix() for p in (ASSETS / 'textures/entity/equipment').rglob('*.png')}
        self.assertEqual(shipped, {f'{folder(stem)}/{stem}' for stem in stems})
        equipment = {p.stem: load(p) for p in (ASSETS / 'equipment').glob('*.json')}
        for name, asset in equipment.items():
            self.assertTrue(asset['layers'], name)
            for kind, layers in asset['layers'].items():
                self.assertIn(kind, folders.values())
                for layer in layers:
                    namespace, path = split(layer['texture'])
                    self.assertEqual(namespace, 'tconstruct')
                    self.assertTrue((ASSETS / f'textures/entity/equipment/{kind}/{path}.png').is_file(), f'{name}: {layer}')
        slots = {'helmet': 'head', 'chestplate': 'chest', 'leggings': 'legs', 'boots': 'feet', 'wings': 'chest'}
        armor = [p for p in (DECLARED / 'item').glob('*.json') if re.fullmatch(r'(travelers|plate|slime)_(helmet|chestplate|leggings|boots)|slime_wings', p.stem)]
        self.assertEqual(len(armor), 13)
        for path in armor:
            equippable = load(path)['components']['minecraft:equippable'] if 'components' in load(path) else None
            if path.stem == 'slime_helmet':
                self.assertIsNone(equippable)  # the slimeskull needs its own skull model; it keeps its template's asset
                continue
            self.assertEqual(equippable['slot'], slots[path.stem.rsplit('_', 1)[1]], path.stem)
            asset = split(equippable['asset_id'])
            self.assertEqual(asset[0], 'tconstruct')
            self.assertIn(asset[1], equipment, path.stem)
            self.assertIn('wings' if path.stem == 'slime_wings' else 'humanoid_leggings' if path.stem.endswith('leggings') else 'humanoid', equipment[asset[1]]['layers'], path.stem)


if __name__ == '__main__':
    unittest.main()
