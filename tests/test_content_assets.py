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
ARMOR_TABLE = ROOT / 'tconstruct/mixins/tconstruct/api/src/armor_textures.rs'
BASE = ROOT / '.local/base-26.3/assets/minecraft'  # a read-only reference checkout; absent on some runners
BASE_DATA = ROOT / '.local/base-26.3/data/minecraft'
DATA = CONTENT / 'data/tconstruct'
# Declared items whose original model is plain (no tool/material loader); the rest wait for per-material rendering.
SIMPLE_ITEMS = {'modifier_crystal', 'pattern', 'venom_bottle', 'raw_cobalt', 'cobalt_shard', 'steel_shard', 'knightmetal_shard', 'sky_slime_ball', 'ichor_slime_ball', 'ender_slime_ball'}
# The one tool whose item is registered under a different name than its definition (the importer's
# MODEL_ALIASES, mirroring item_for_definition in tool_definitions.rs): the slime chestplate's
# definition is slime_chestplate, its item slimy_chestplate.
TOOL_ITEM_NAMES = {'slime_chestplate': 'slimy_chestplate'}


def load(path):
    return json.loads(path.read_text(encoding='utf-8'))


def item_definition(tool):
    """The item definition file for a tool definition name, through TOOL_ITEM_NAMES."""
    return ASSETS / 'items' / f'{TOOL_ITEM_NAMES.get(tool, tool)}.json'


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
            if 'loader' in model:
                # Connected glass ships in the host's Fusion form (the DATA-W2 conversion of the Mantle
                # connection models): that loader, in its connecting type, is the one loader a model may carry.
                self.assertEqual((model['loader'], model.get('type')), ('fusion:model', 'fusion:connecting'), path)
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
            blockstate = load(ASSETS / 'blockstates' / f'{name}.json')
            # a multipart block state (the channel) applies a model, or a list of them, per part
            applied = list(blockstate['variants'].values()) if 'variants' in blockstate else [
                apply for part in blockstate['multipart'] for apply in (part['apply'] if isinstance(part['apply'], list) else [part['apply']])]
            self.assertTrue(applied, name)
            for variant in applied:
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
        """Each tool, armor piece and shield is a tree over its original states whose leaves are one plain model
        (staffs, flint and brick) or a composite of selects whose index is a material slot of its tool definition and
        whose cases are shipped material ids, then the modifier overlays; every model resolves."""
        materials = {p.stem for p in MATERIALS.glob('*.json')}
        tools = sorted(p.stem for p in TOOLS.glob('*.json'))
        self.assertEqual(len(tools), 45)
        self.used_overlays = set()
        for tool in tools:
            definition = load(TOOLS / f'{tool}.json')
            slots = sum(len(m.get('parts') or m.get('stat_types') or []) for m in definition['modules'] if m['type'] in ('tconstruct:part_stats', 'tconstruct:material_stats'))
            leaves = list(self.walk(load(item_definition(tool))['model'], tool))
            self.assertTrue(leaves, tool)
            for leaf in leaves:
                for layer in (leaf['models'] if leaf['type'] == 'minecraft:composite' else [leaf]):
                    self.check_layer(layer, tool, slots, materials)
        flags = flags_rs()
        self.assertEqual(self.used_overlays, set(range(len(flags['OVERLAYS']))), 'every table entry is drawn by some tool')
        self.assertEqual(flags['OVERLAYS'], sorted(flags['OVERLAYS']))
        for modifier in flags['OVERLAYS']:
            name = modifier.removeprefix('tconstruct:')
            # a modifier the original defines in code (no data definition, e.g. ambidextrous) is still a real one: its recipe ships
            known = (MODIFIERS / f'{name}.json').is_file() or any((CONTENT / 'data/tconstruct/recipes/tools/modifiers').rglob(f'{name}.json'))
            self.assertTrue(known, modifier)

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
        return {c['index'] for c in self.conditions(load(item_definition(tool))['model'], 'minecraft:custom_model_data')}

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
            broken = self.conditions(load(item_definition(tool))['model'], 'minecraft:broken')
            self.assertEqual(bool(broken), tool not in stateless, tool)
        pickaxe = self.conditions(load(ASSETS / 'items/pickaxe.json')['model'], 'minecraft:broken')[0]
        self.assertIn('tconstruct:item/tool/pickaxe/head_broken', self.textures_of(pickaxe['on_true']))
        self.assertNotIn('tconstruct:item/tool/pickaxe/head_broken', self.textures_of(pickaxe['on_false']))
        self.assertIn('tconstruct:item/tool/pickaxe/head', self.textures_of(pickaxe['on_false']))

    def test_drawing_blocking_and_ammo_states(self):
        """The original's charge thresholds, as use_duration dispatches under `using_item`; crossbow ammo and the use animation are flags;
        the rod's cast is the host's property."""
        def thresholds(tool):
            nodes = [n for n in self.nodes(load(item_definition(tool))['model']) if n['type'] == 'minecraft:range_dispatch']
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

    def declared(self):
        return {p.stem for p in (DECLARED / 'block').glob('*.json')}, {p.stem for p in (DECLARED / 'item').glob('*.json')}

    def test_every_declared_block_and_item_has_its_original_loot_and_names(self):
        blocks, items = self.declared()
        lang = load(ASSETS / 'lang/en_us.json')
        # The original ships no loot table for a fluid block (the fluid itself drops nothing) nor for glow
        # (placed by the staffs, never mined), and its lang names a wall head by the standing head's key:
        # it has no wall head keys at all. Every other declared block drops by the original's table.
        no_loot = {name for name in blocks if name.endswith('_fluid')} | {'glow'}
        for name in blocks:
            if not name.endswith('_wall_head'):
                self.assertIn(f'block.tconstruct.{name}', lang, name)
            table = DATA / 'loot_table/blocks' / f'{name}.json'
            if name in no_loot:
                self.assertFalse(table.exists(), name)
                continue
            loot = load(table)
            # A wall variant (head, sign, hanging sign) drops through its standing block's table, as the
            # original defines the wall blocks' loot in code: its table is that table, sequence included.
            sequence = name.replace('_wall_', '_') if '_wall_' in name else name
            self.assertEqual((loot['type'], loot['random_sequence']), ('minecraft:block', f'tconstruct:blocks/{sequence}'))
            for entry in self.loot_entries(loot):
                namespace, path = split(entry['name'])
                self.assertIn(path, blocks | items if namespace == 'tconstruct' else {path}, f'{name} drops {entry["name"]}')
                if namespace == 'minecraft' and BASE.is_dir():
                    self.assertTrue((BASE / 'items' / f'{path}.json').is_file(), f'{name} drops {entry["name"]}')
        for name in SIMPLE_ITEMS:
            self.assertIn(f'item.tconstruct.{name}', lang, name)
        self.assertEqual({p.stem for p in (DATA / 'loot_table/blocks').glob('*.json')}, blocks - no_loot)

    def loot_entries(self, node):
        for pool in node.get('pools', []):
            stack = list(pool['entries'])
            while stack:
                entry = stack.pop()
                stack += entry.get('children', [])
                if entry['type'] == 'minecraft:item':
                    yield entry

    def test_imported_loot_has_the_shape_of_vanilla_26_3_loot_for_the_same_drops(self):
        """The 1.20.1 tables are converted to the form the host reads: an ore is vanilla's diamond ore under other names, a block that
        drops itself is vanilla's slime block."""
        if not BASE_DATA.is_dir():
            self.skipTest('no base data')

        def renamed(text, names):
            for old, new in names.items():
                text = text.replace(old, new)
            return json.loads(text)
        ore = (BASE_DATA / 'loot_table/blocks/diamond_ore.json').read_text(encoding='utf-8')
        self.assertEqual(load(DATA / 'loot_table/blocks/cobalt_ore.json'), renamed(ore, {'minecraft:diamond_ore': 'tconstruct:cobalt_ore', 'minecraft:diamond': 'tconstruct:raw_cobalt', 'minecraft:blocks/diamond_ore': 'tconstruct:blocks/cobalt_ore'}))
        block = (BASE_DATA / 'loot_table/blocks/slime_block.json').read_text(encoding='utf-8')
        for name in ('sky_slime', 'ichor_slime', 'ender_slime', 'raw_cobalt_block', 'earth_slime_dirt', 'sky_slime_dirt', 'ichor_slime_dirt', 'ender_slime_dirt'):
            self.assertEqual(load(DATA / f'loot_table/blocks/{name}.json'), renamed(block, {'minecraft:slime_block': f'tconstruct:{name}', 'minecraft:blocks/slime_block': f'tconstruct:blocks/{name}'}), name)
        # Congealed slime: itself with silk touch, else four slime balls of its type (the original's alternatives, with the host's silk touch condition).
        for slime in ('earth', 'sky', 'ichor', 'ender'):
            alternatives = load(DATA / f'loot_table/blocks/{slime}_congealed_slime.json')['pools'][0]['entries'][0]
            silk, drop = alternatives['children']
            self.assertEqual((silk['condition'], silk['name']), ('minecraft:tool/can_silk_touch', f'tconstruct:{slime}_congealed_slime'))
            self.assertEqual((drop['name'], drop['modifier'][0]), ('minecraft:slime_ball' if slime == 'earth' else f'tconstruct:{slime}_slime_ball', {'type': 'minecraft:set_count', 'count': 4}))

    def test_cobalt_generates_in_the_nether_through_its_own_features(self):
        """TinkerWorld's two ore features and the add_features biome modifier: placed features name shipped features, which place the
        declared ore block into netherrack, and the modifier adds exactly those to the nether."""
        blocks, _ = self.declared()
        features = {p.stem: load(p) for p in (DATA / 'worldgen/feature').glob('*.json')}
        placed = {p.stem: load(p) for p in (DATA / 'worldgen/placed_feature').glob('*.json')}
        cobalt = {'cobalt_ore_small', 'cobalt_ore_large'}
        self.assertEqual((set(features) & cobalt, set(placed) & cobalt), (cobalt, cobalt))
        self.assertEqual((features['cobalt_ore_small']['size'], features['cobalt_ore_large']['size']), (4, 6))
        self.assertIn('cobalt_ore', blocks)
        for name in sorted(cobalt):
            feature = features[name]
            self.assertEqual(placed[name]['feature'], f'tconstruct:{name}')
            self.assertEqual([t['state'] for t in feature['targets']], ['tconstruct:cobalt_ore'])
            self.assertEqual([t['target'] for t in feature['targets']], [{'predicate_type': 'minecraft:block_match', 'block': 'minecraft:netherrack'}])
            self.assertNotIn('config', feature)  # 26.3 writes a feature's settings in its own body
            self.assertEqual(placed[name]['placement'][-1], {'type': 'minecraft:biome'})
        if BASE_DATA.is_dir():
            self.assertEqual(set(features['cobalt_ore_small']), set(load(BASE_DATA / 'worldgen/feature/ore_nether_gold.json')))
            self.assertEqual(set(placed['cobalt_ore_small']), set(load(BASE_DATA / 'worldgen/placed_feature/ore_gold_nether.json')))
            self.assertTrue((BASE_DATA / 'tags/worldgen/biome/is_nether.json').is_file())
        modifiers = {p.stem: load(p) for p in (DATA / 'neoforge/biome_modifier').glob('*.json')}
        self.assertEqual(modifiers['cobalt_ore'], {'type': 'neoforge:add_features', 'biomes': '#minecraft:is_nether', 'features': ['tconstruct:cobalt_ore_small', 'tconstruct:cobalt_ore_large'], 'step': 'underground_decoration'})

    def test_world_blocks_carry_the_originals_numbers_and_tool_tags(self):
        """The declarations of TinkerWorld's registrations (hardness, resistance, friction, light, tool need) and the harvest tags."""
        def properties(name):
            return load(DECLARED / 'block' / f'{name}.json')['properties']

        keys = ('hardness', 'explosion_resistance', 'requires_tool')
        self.assertEqual({k: properties('cobalt_ore')[k] for k in keys}, {'hardness': 10.0, 'explosion_resistance': 10.0, 'requires_tool': True})
        self.assertEqual({k: properties('raw_cobalt_block')[k] for k in keys}, {'hardness': 6.0, 'explosion_resistance': 7.0, 'requires_tool': True})
        for slime, light in (('earth', None), ('sky', None), ('ichor', 10), ('ender', None)):
            congealed = properties(f'{slime}_congealed_slime')
            self.assertEqual((congealed['hardness'], congealed['friction'], congealed.get('light')), (0.5, 0.5, light), slime)
            self.assertEqual(properties(f'{slime}_slime_dirt')['hardness'], 1.9, slime)
            if slime != 'earth':
                sticky = load(DECLARED / f'block/{slime}_slime.json')
                self.assertEqual((sticky['template'], sticky['properties']['friction'], sticky['properties']['occludes'], sticky['properties'].get('light')), ('minecraft:slime_block', 0.8, False, light), slime)
        self.assertFalse((DECLARED / 'block/earth_slime.json').exists())  # vanilla's slime block
        tags = CONTENT / 'data/minecraft/tags/block'

        def members(path):
            return set(load(tags / path)['values'])
        dirt = {f'tconstruct:{t}_slime_dirt' for t in ('earth', 'sky', 'ichor', 'ender')}
        congealed = {f'tconstruct:{t}_congealed_slime' for t in ('earth', 'sky', 'ichor', 'ender')}
        self.assertTrue({'tconstruct:cobalt_ore', 'tconstruct:raw_cobalt_block'} <= members('mineable/pickaxe.json'))
        self.assertTrue(dirt | congealed <= members('mineable/shovel.json'))
        self.assertTrue({'tconstruct:cobalt_ore', 'tconstruct:raw_cobalt_block', 'tconstruct:ichor_slime_dirt'} <= members('needs_iron_tool.json'))
        self.assertIn('tconstruct:earth_slime_dirt', members('needs_stone_tool.json'))
        self.assertIn('tconstruct:ender_slime_dirt', members('needs_diamond_tool.json'))
        self.assertTrue({'#tconstruct:congealed_slime', '#tconstruct:slimy_soil'} <= members('enderman_holdable.json'))
        # Every entry names a declared block or a shipped Tinkers tag.
        blocks, _ = self.declared()
        for path in tags.rglob('*.json'):
            for entry in load(path)['values']:
                value = entry if isinstance(entry, str) else entry['id']
                if value.startswith('#'):
                    self.assertTrue((CONTENT / 'data/tconstruct/tags/block' / f'{value.split(":", 1)[1]}.json').is_file(), f'{path.name}: {value}')
                else:
                    self.assertIn(value.removeprefix('tconstruct:'), blocks, f'{path.name}: {value}')

    def test_slime_block_models_keep_the_originals_texture_and_vanilla_parent(self):
        """Sky and ender slime reuse vanilla's slime block shape with their own texture; ichor adds the lava core; item models never carry the 1.20 render_type key."""
        for slime in ('sky', 'ender'):
            model = load(ASSETS / f'models/block/slime/sticky/{slime}.json')
            self.assertEqual(model['parent'], 'minecraft:block/slime_block')
            self.assertEqual(model['textures']['texture'], f'tconstruct:block/slime/storage/{slime}')
        ichor = load(ASSETS / 'models/block/slime/sticky/ichor.json')
        self.assertEqual(ichor['textures']['inner'], 'minecraft:block/lava_still')
        self.assertEqual(len(ichor['elements']), 2)
        # Item models only: the originals' item models are parent-only stubs with no layer
        # key, and the port's flat item models need none. Block models are exempt because
        # the host honors NeoForge's render_type (horizon-assets model.rs) and the
        # original platform block models require "minecraft:cutout" — without it the
        # platform renders as an opaque plate instead of a see-through lattice.
        for path in (ASSETS / 'models' / 'item').rglob('*.json'):
            self.assertNotIn('render_type', load(path), path)

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
        # GUI textures are drawn by the screens' Rust code, not by models: the sprite names the sources use.
        sources = ''.join(path.read_text(encoding='utf-8') for path in (ROOT / 'tconstruct/mixins/tconstruct/api/src').glob('*.rs'))
        used |= {f'gui/{name}' for name in re.findall(r'"tconstruct:gui/([a-z_/]+)"', sources)}
        # Fluid sprites are drawn by the block-entity renderers and the tank windows under the names of
        # Mantle's fluid texture table (render_data.rs), not by models: the sprite names the sources use.
        used |= {f'fluid/{name}' for name in re.findall(r'"tconstruct:fluid/([a-z_/]+)"', sources)}
        # Three fluid sprites are named by convention, not by a literal: the base fluid model's flowing
        # counterpart (its still is model-named), and the inverted flowing sprites of the two gas fluids
        # (cinderslime, ichor), which Mantle names from the flowing sprite of a fluid lighter than air.
        used |= {'fluid/liquid/flowing', 'fluid/molten/alloy/cinderslime/flowing_inverted',
                 'fluid/slime/ichor/flowing_inverted'}
        # Connected-glass sprites are picked by Mantle's connected-texture rule from the folder of a
        # `connection` model's block texture: the fifteen direction-combination sprites, and, in the two
        # full-glass folders (foundry glass, seared glass), the `normal` sprite of an unconnected face.
        edges = ('d', 'dl', 'dlr', 'dr', 'l', 'lr', 'r', 'u', 'ud', 'udl', 'udlr', 'udr', 'ul', 'ulr', 'ur')
        for path in (ASSETS / 'models').rglob('*.json'):
            model = load(path)
            if 'connection' not in model:
                continue
            for texture in model.get('textures', {}).values():
                namespace, name = split(texture)
                if namespace == 'tconstruct' and not name.endswith('_top'):
                    used |= {f'{name}/{edge}' for edge in edges}
                    if name in ('block/foundry/glass', 'block/smeltery/seared_glass'):
                        used.add(f'{name}/normal')
        # Particle sprites are named by the shipped particle definitions (assets/tconstruct/particles/*.json),
        # whose texture ids are relative to textures/particle/: the host's particle loader reads them from there.
        for path in (ASSETS / 'particles').glob('*.json'):
            used |= {f'particle/{split(t)[1]}' for t in load(path).get('textures', []) if split(t)[0] == 'tconstruct'}
        # Trim palettes are read by the trim renderer under the trim material's name (the importer's
        # copy_trim_palettes): one palette per shipped trim material, named by the material.
        used |= {f'trims/color_palettes/{p.stem}' for p in (CONTENT / 'data/tconstruct/trim_material').glob('*.json')}
        # Worn equipment textures are picked by the layer table and skull textures by the skull rendering
        # code, not by models: the original references both by convention, so neither is model-used.
        # Modifier icons are the same kind: the icon manager names them from the modifier id by
        # convention (gui/modifiers/<category>/<modifier>), with tinkering/modifier_icons.json holding
        # the per-modifier overrides, so the shipped icon tree is not model-used either. The mob effect
        # icons and the slime grass colormaps are whole trees of the same kind: the host resolves an
        # effect's icon from its id alone (amplifier variants and the charging bar included) and the
        # colour providers sample the colormaps per grass type, so the importer copies both trees whole
        # rather than by reachability. A gui/modifiers sprite the sources name literally (the worktable
        # book set) is not convention-named, so it stays in the accounting.
        present = set()
        for path in (ASSETS / 'textures').rglob('*.png'):
            name = path.relative_to(ASSETS / 'textures').with_suffix('').as_posix()
            if name.startswith(('entity/equipment/', 'entity/skull/', 'mob_effect/', 'colormap/')):
                continue
            if name.startswith('gui/modifiers/') and name not in used:
                continue
            present.add(name)
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
        armor = [p for p in (DECLARED / 'item').glob('*.json') if re.fullmatch(r'(travelers|plate|slime|slimy)_(helmet|chestplate|leggings|boots)|slime_wings', p.stem)]
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
