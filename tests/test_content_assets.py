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
FLAGS_TABLE = ROOT / 'tconstruct/mixins/tconstruct/api/src/model_flags.rs'
MODIFIERS = CONTENT / 'data/tconstruct/tinkering/modifiers'
BASE = ROOT / '.local/base-26.3/assets/minecraft'  # a read-only reference checkout; absent on some runners
# Declared items whose original model is plain (no tool/material loader); the rest wait for per-material rendering.
SIMPLE_ITEMS = {'modifier_crystal', 'pattern', 'venom_bottle'}


def load(path):
    return json.loads(path.read_text(encoding='utf-8'))


def split(reference):
    namespace, _, path = reference.partition(':') if ':' in reference else ('minecraft', '', reference)
    return namespace, path


def flags_rs():
    """The custom_model_data.flags positions the importer wrote for the Rust side (model_flags.rs)."""
    text = FLAGS_TABLE.read_text(encoding='utf-8')
    positions = {name: int(value) for name, value in re.findall(r'const ([A-Z_]+): usize = (\d+);', text)}
    return {**positions, 'OVERLAYS': re.findall(r'"(tconstruct:[a-z_]+)"', text.split('const OVERLAYS')[1])}


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

    def walk(self, node, tool):
        """Yield every leaf (a model, a composite, or a material select outside one) of a tool's item model tree, checking
        each branch: the state conditions the host reads, the custom_model_data flag positions, the charge dispatch."""
        kind = node['type']
        if kind == 'minecraft:select' and node['property'] == 'minecraft:display_context':
            self.assertEqual([case['when'] for case in node['cases']], ['gui'], tool)
            yield from self.walk(node['cases'][0]['model'], tool)
            yield from self.walk(node['fallback'], tool)
        elif kind == 'minecraft:condition':
            if node['property'] == 'minecraft:custom_model_data':
                flags = flags_rs()
                self.assertIn(node['index'], (flags['BLOCKING'], flags['SPEAR'], flags['ARROW'], flags['FIREWORK']), tool)
            else:
                self.assertIn(node['property'], ('minecraft:broken', 'minecraft:using_item', 'minecraft:fishing_rod/cast'), tool)
            self.assertNotEqual(node['on_true'], node['on_false'], tool)
            yield from self.walk(node['on_true'], tool)
            yield from self.walk(node['on_false'], tool)
        elif kind == 'minecraft:range_dispatch':
            self.assertEqual((node['property'], node['scale']), ('minecraft:use_duration', 0.05), tool)
            thresholds = [entry['threshold'] for entry in node['entries']]
            self.assertEqual(thresholds, sorted(set(thresholds)), tool)
            self.assertGreater(thresholds[0], 0, tool)
            for entry in node['entries']:
                yield from self.walk(entry['model'], tool)
            yield from self.walk(node['fallback'], tool)
        else:
            yield node

    def check_layer(self, layer, tool, slots, materials):
        if layer['type'] == 'minecraft:model':
            self.assertTrue(self.resolves('models', 'json', layer['model']), tool)
        elif layer['type'] == 'minecraft:select':
            self.assertEqual(layer['property'], 'minecraft:custom_model_data', tool)
            self.assertLess(layer['index'], slots, tool)
            self.assertTrue(self.resolves('models', 'json', layer['fallback']['model']), tool)
            seen = set()
            for case in layer['cases']:
                self.assertIn(case['when'], materials, tool)
                self.assertNotIn(case['when'], seen, tool)
                seen.add(case['when'])
                self.assertTrue(self.resolves('models', 'json', case['model']['model']), f'{tool}: {case["when"]}')
        else:  # a modifier overlay: shown while the tool's flag for that modifier is set
            flags = flags_rs()
            self.assertEqual((layer['type'], layer['property']), ('minecraft:condition', 'minecraft:custom_model_data'), tool)
            self.assertTrue(flags['OVERLAY_BASE'] <= layer['index'] < flags['OVERLAY_BASE'] + len(flags['OVERLAYS']), tool)
            self.assertEqual(layer['on_false'], {'type': 'minecraft:empty'}, tool)
            self.assertTrue(self.resolves('models', 'json', layer['on_true']['model']), tool)
            self.used_overlays.add(layer['index'] - flags['OVERLAY_BASE'])

    def test_every_layered_tool_composes_material_layers_by_slot(self):
        """Each non-armor tool (shields included) is a tree over its original states whose leaves are one plain model
        (staffs, flint and brick) or a composite of selects whose index is a material slot of its tool definition and
        whose cases are shipped material ids, then the modifier overlays; every model resolves."""
        materials = {p.stem for p in MATERIALS.glob('*.json')}
        armor = ('plate_', 'slime_', 'travelers_')
        tools = sorted(p.stem for p in TOOLS.glob('*.json') if not p.stem.startswith(armor) or p.stem.endswith('_shield'))
        self.assertEqual(len(tools), 31)
        self.used_overlays = set()
        for tool in tools:
            definition = load(TOOLS / f'{tool}.json')
            slots = sum(len(m.get('parts') or m.get('stat_types') or []) for m in definition['modules'] if m['type'] in ('tconstruct:part_stats', 'tconstruct:material_stats'))
            leaves = list(self.walk(load(ASSETS / 'items' / f'{tool}.json')['model'], tool))
            self.assertTrue(leaves, tool)
            for leaf in leaves:
                for layer in (leaf['models'] if leaf['type'] == 'minecraft:composite' else [leaf]):
                    self.check_layer(layer, tool, slots, materials)
        flags = flags_rs()
        self.assertEqual(self.used_overlays, set(range(len(flags['OVERLAYS']))), 'every table entry is drawn by some tool')
        self.assertEqual(flags['OVERLAYS'], sorted(flags['OVERLAYS']))
        for modifier in flags['OVERLAYS']:
            self.assertTrue((MODIFIERS / f'{modifier.removeprefix("tconstruct:")}.json').is_file(), modifier)

    def nodes(self, node):
        """Every node of an item model tree, outermost first."""
        yield node
        for key in ('on_true', 'on_false', 'fallback'):
            if key in node:
                yield from self.nodes(node[key])
        for child in node.get('cases', []) + node.get('entries', []):
            yield from self.nodes(child['model'])

    def conditions(self, node, prop):
        return [n for n in self.nodes(node) if n['type'] == 'minecraft:condition' and n['property'] == prop]

    def flag_positions(self, tool):
        return {c['index'] for c in self.conditions(load(ASSETS / 'items' / f'{tool}.json')['model'], 'minecraft:custom_model_data')}

    def textures_of(self, node):
        """The texture references of the base sprite of every layer under a (sub)tree."""
        found = set()
        for leaf in self.walk(node, 'tree'):
            for layer in (leaf['models'] if leaf['type'] == 'minecraft:composite' else [leaf]):
                model = layer['fallback'] if layer['type'] == 'minecraft:select' else layer['on_true'] if layer['type'] == 'minecraft:condition' else layer
                found |= set(load(ASSETS / 'models' / f'{split(model["model"])[1]}.json')['textures'].values())
        return found

    def test_broken_tools_show_their_broken_sprites(self):
        """`minecraft:broken` is the host's damage >= max_damage - 1; a broken tool's stack carries damage = durability and max_damage = durability + 1
        (tool_damage::fake_max_damage), an unbroken one at most durability - 1 of durability. The original gives no broken model to arrows,
        shuriken and throwing axes."""
        stateless = {'arrow', 'shuriken', 'throwing_axe'}
        tools = {p.stem for p in TOOLS.glob('*.json') if not p.stem.startswith(('plate_', 'slime_', 'travelers_')) or p.stem.endswith('_shield')}
        for tool in tools:
            broken = self.conditions(load(ASSETS / 'items' / f'{tool}.json')['model'], 'minecraft:broken')
            self.assertEqual(bool(broken), tool not in stateless, tool)
        pickaxe = self.conditions(load(ASSETS / 'items/pickaxe.json')['model'], 'minecraft:broken')[0]
        self.assertIn('tconstruct:item/tool/pickaxe/head_broken', self.textures_of(pickaxe['on_true']))
        self.assertNotIn('tconstruct:item/tool/pickaxe/head_broken', self.textures_of(pickaxe['on_false']))
        self.assertIn('tconstruct:item/tool/pickaxe/head', self.textures_of(pickaxe['on_false']))

    def test_drawing_blocking_and_ammo_states(self):
        """The original's charge thresholds, as use_duration dispatches under `using_item`; crossbow ammo and the use animation are flags;
        the rod's cast is the host's property."""
        def thresholds(tool):
            nodes = [n for n in self.nodes(load(ASSETS / 'items' / f'{tool}.json')['model']) if n['type'] == 'minecraft:range_dispatch']
            return sorted({tuple(entry['threshold'] for entry in node['entries']) for node in nodes})
        self.assertEqual(thresholds('longbow'), [(0.05, 0.6, 1.0)])
        self.assertEqual(thresholds('crossbow'), [(0.05, 0.58, 1.0)])
        self.assertEqual(thresholds('swasher'), [(0.05, 1.0), (1.0,)])
        self.assertEqual(thresholds('earth_staff'), [(0.05, 0.4, 0.7, 0.9, 1.0), (0.4, 0.7, 0.9, 1.0)])
        self.assertEqual(thresholds('melting_pan'), [(1.0,)])
        self.assertEqual(thresholds('pickaxe'), [])
        flags = flags_rs()
        self.assertTrue({flags['ARROW'], flags['FIREWORK']} <= self.flag_positions('crossbow'))
        self.assertTrue(self.conditions(load(ASSETS / 'items/fishing_rod.json')['model'], 'minecraft:fishing_rod/cast'))
        for tool in ('pickaxe', 'sword', 'plate_shield', 'travelers_shield', 'fishing_rod', 'longbow'):
            self.assertIn(flags['BLOCKING'], self.flag_positions(tool), tool)
        for tool in ('sword', 'dagger', 'cleaver', 'javelin'):
            self.assertIn(flags['SPEAR'], self.flag_positions(tool), tool)
        self.assertFalse(self.conditions(load(ASSETS / 'items/shuriken.json')['model'], 'minecraft:using_item'))
        # Held in use and blocking shows the blocking display; the same sprites under a model whose parent carries the transforms.
        pickaxe = load(ASSETS / 'items/pickaxe.json')['model']
        blocking = [n for n in self.conditions(pickaxe, 'minecraft:custom_model_data') if n['index'] == flags['BLOCKING']][0]
        self.assertNotEqual(blocking['on_true'], blocking['on_false'])
        self.assertTrue(any('item/tool_state/' in load(ASSETS / 'models' / f'{split(m)[1]}.json')['parent'] for m in
                            [layer['fallback']['model'] for layer in blocking['on_true']['models'] if layer['type'] == 'minecraft:select']))

    def test_large_tools_draw_32px_quads_outside_the_gui(self):
        """The original's BakedToolModel uses the small model in the GUI only; the large one is 32 units wide, shifted by large_offset."""
        large = {'broad_axe', 'cleaver', 'earth_staff', 'ender_staff', 'excavator', 'ichor_staff', 'javelin', 'longbow', 'plate_shield', 'scythe', 'sky_staff', 'sledge_hammer', 'vein_hammer'}

        def sizes(node):
            result = set()
            for leaf in self.walk(node, 'tree'):
                for layer in (leaf['models'] if leaf['type'] == 'minecraft:composite' else [leaf]):
                    model = layer['fallback'] if layer['type'] == 'minecraft:select' else layer['on_true'] if layer['type'] == 'minecraft:condition' else layer
                    elements = load(ASSETS / 'models' / f'{split(model["model"])[1]}.json').get('elements', [{'from': [0, 0, 0], 'to': [16, 16, 0]}])
                    result |= {(e['to'][0] - e['from'][0], e['to'][1] - e['from'][1]) for e in elements}
            return result
        for path in (ASSETS / 'items').glob('*.json'):
            model = load(path)['model']
            is_large = model['type'] == 'minecraft:select' and model['property'] == 'minecraft:display_context'
            self.assertEqual(is_large, path.stem in large, path.stem)
            if is_large:
                self.assertEqual(sizes(model['cases'][0]['model']), {(16, 16)}, path.stem)
                self.assertEqual(sizes(model['fallback']), {(32, 32)}, path.stem)
        limb = load(ASSETS / 'models/item/tool_layer/item_base_bow/longbow/large/limb_bottom.json')['elements'][0]
        self.assertEqual((limb['from'][:2], limb['to'][:2]), ([-7.5, -8.5], [24.5, 23.5]))

    def test_every_content_path_is_a_plain_host_path(self):
        # The host refuses the whole content mod when one path has a part outside [a-z0-9_.-] (measured on r3i: the mod's
        # blocks and items were not registered because of assets/tconstruct/LICENSE).
        bad = [p.relative_to(CONTENT).as_posix() for p in CONTENT.rglob('*')
               if p.is_file() and p.relative_to(CONTENT).parts[0] in ('assets', 'data')
               and not all(re.fullmatch(r'[a-z0-9_.-]+', part) for part in p.relative_to(CONTENT).parts)]
        self.assertEqual(bad, [])

    def test_every_texture_is_used(self):
        used = set()
        for path in (ASSETS / 'models').rglob('*.json'):
            used |= {split(t)[1] for t in load(path).get('textures', {}).values() if split(t)[0] == 'tconstruct'}
        present = {p.relative_to(ASSETS / 'textures').with_suffix('').as_posix() for p in (ASSETS / 'textures').rglob('*.png')}
        self.assertEqual(present, used)


if __name__ == '__main__':
    unittest.main()
