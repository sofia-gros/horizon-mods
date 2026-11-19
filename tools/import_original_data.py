#!/usr/bin/env python3
"""Copy the original mod's generated tinkering data into the clean content root.

Source: the read-only checkout at .local/tconstruct-source (revision pinned by
tconstruct/compatibility.json). Files are copied byte for byte, never edited, so the content
root stays a mechanical projection of the original. Other-mod integration is left out:
the minotaur axe and every material whose condition depends on external ingot tags.
"""
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/tinkering'
TARGET = ROOT / 'tconstruct/content/data/tconstruct/tinkering'
FOLDERS = ['tool_definitions', 'materials/definition', 'materials/stats', 'materials/traits', 'modifiers']
TAG_FOLDERS = ['tags/materials', 'tags/modifiers']
EXCLUDED_TOOLS = {'minotaur_axe'}
# The original's Forge-era tag folders, renamed to the singular names the pinned host reads.
TAG_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/tags'
TAG_TARGET = ROOT / 'tconstruct/content/data/tconstruct/tags'
TAG_FOLDER_NAMES = {'items': 'item', 'blocks': 'block', 'entity_types': 'entity_type'}


def external(condition):
    """True when a material condition is about ingots another mod must supply."""
    text = json.dumps(condition)
    return 'mantle:tag_filled' in text or 'forge:mod_loaded' in text


def write_creative_parts(skipped):
    """creative_parts.rs: every tool part with the visible materials that have its stat type.

    Same selection and order as IMaterialItem.addVariants: visible (not hidden) concrete materials by
    tier, sort order, path and namespace, each kept when its stats include the part's stat type.
    """
    definitions = {}
    for path in (TARGET / 'materials/definition').glob('*.json'):
        data = json.loads(path.read_text(encoding='utf-8'))
        if 'redirect' in data or data.get('hidden', False):
            continue
        definitions[path.stem] = data
    order = sorted(definitions, key=lambda name: (definitions[name].get('tier', 0), definitions[name].get('sortOrder', 100), name, 'tconstruct'))
    stats = {name: set(json.loads((TARGET / 'materials/stats' / f'{name}.json').read_text(encoding='utf-8')).get('stats', {}))
             for name in order if (TARGET / 'materials/stats' / f'{name}.json').exists()}
    source = (ROOT / 'tconstruct/mixins/tconstruct/api/src/tool_parts.rs').read_text(encoding='utf-8')
    table = source.split('const PARTS')[1].split('];')[0]
    parts = re.findall(r'\("([a-z_]+)", "([a-z_]+)"\)', table)
    # RepairKitItem is a material item with its own stat type but is not a ToolPart.
    parts.append(('repair_kit', 'repair_kit'))
    lines = ['//! Tool parts with the visible shipped materials that can make them, in creative-tab order.',
             '//! Generated from the original data by tools/import_original_data.py; do not edit by hand.', '',
             'pub(crate) const PARTS: &[(&str, &[&str])] = &[']
    for item, stat in parts:
        materials = [name for name in order if f'tconstruct:{stat}' in stats.get(name, set())]
        lines.append(f'    ("{item}", &[' + ', '.join(f'"{name}"' for name in materials) + ']),')
    lines.append('];')
    (ROOT / 'tconstruct/mixins/tconstruct/api/src/creative_parts.rs').write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')


ARMOR_TOOLS = ['travelers_helmet', 'travelers_chestplate', 'travelers_leggings', 'travelers_boots', 'travelers_shield',
               'plate_helmet', 'plate_chestplate', 'plate_leggings', 'plate_boots', 'plate_shield',
               'slime_helmet', 'slime_chestplate', 'slime_leggings', 'slime_boots', 'slime_wings']


def write_creative_tools():
    """creative_tools.rs: the armor and shield stacks of the Tools tab, as ToolBuildHandler.addVariants lists them.

    Per tool, one material list for each visible material (same order as the parts tab) that has any of the
    tool's stat types; slots whose stat type the material lacks take the first visible material with it
    (createSingleMaterial / MaterialRegistry.firstWithStatType).
    """
    definitions = {}
    for path in (TARGET / 'materials/definition').glob('*.json'):
        data = json.loads(path.read_text(encoding='utf-8'))
        if 'redirect' in data or data.get('hidden', False):
            continue
        definitions[path.stem] = data
    order = sorted(definitions, key=lambda name: (definitions[name].get('tier', 0), definitions[name].get('sortOrder', 100), name, 'tconstruct'))
    stats = {name: set(json.loads((TARGET / 'materials/stats' / f'{name}.json').read_text(encoding='utf-8')).get('stats', {}))
             for name in order if (TARGET / 'materials/stats' / f'{name}.json').exists()}
    source = (ROOT / 'tconstruct/mixins/tconstruct/api/src/tool_parts.rs').read_text(encoding='utf-8')
    table = source.split('const PARTS')[1].split('];')[0]
    part_stat = dict(re.findall(r'\("([a-z_]+)", "([a-z_]+)"\)', table))
    lines = ['//! Armor and shield stacks of the Tools creative tab: per tool, the material of each slot for every entry.',
             '//! Generated from the original data by tools/import_original_data.py; do not edit by hand.', '',
             'pub(crate) const ARMOR: &[(&str, &[&[&str]])] = &[']
    for tool in ARMOR_TOOLS:
        modules = json.loads((TARGET / 'tool_definitions' / f'{tool}.json').read_text(encoding='utf-8'))['modules']
        required = []
        for module in modules:
            if module['type'] == 'tconstruct:part_stats':
                required = [part_stat[part.split(':')[1]] for part in module['parts']]
            elif module['type'] == 'tconstruct:material_stats':
                required = [(entry if isinstance(entry, str) else entry['stat']).split(':')[1] for entry in module['stat_types']]
        has = lambda name, stat: f'tconstruct:{stat}' in stats.get(name, set())
        first = [next(name for name in order if has(name, stat)) for stat in required]
        entries = []
        for name in order:
            if any(has(name, stat) for stat in required):
                entries.append([name if has(name, stat) else first[i] for i, stat in enumerate(required)])
        rows = ', '.join('&[' + ', '.join(f'"{m}"' for m in entry) + ']' for entry in entries)
        lines.append(f'    ("{tool}", &[{rows}]),')
    lines.append('];')
    (ROOT / 'tconstruct/mixins/tconstruct/api/src/creative_tools.rs').write_text(chr(10).join(lines) + chr(10), encoding='utf-8', newline=chr(10))


RECIPE_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes/tools/materials'
RECIPE_TARGET = ROOT / 'tconstruct/content/data/tconstruct/recipes/tools/materials'


def copy_material_recipes(skipped_materials):
    """The item -> material recipes (type tconstruct:material), byte for byte. The fluid and melting
    recipes in the same folder belong to the smeltery domain. A recipe whose material was left out as
    other-mod integration is left out with it."""
    if RECIPE_TARGET.exists():
        shutil.rmtree(RECIPE_TARGET)
    count = 0
    for path in sorted(RECIPE_SOURCE.rglob('*.json')):
        data = json.loads(path.read_text(encoding='utf-8'))
        if data.get('type') != 'tconstruct:material':
            continue
        if data['material'].split(':')[-1].split('#')[0] in skipped_materials:
            continue
        destination = RECIPE_TARGET / path.relative_to(RECIPE_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    return count


MODIFIER_RECIPE_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes/tools/modifiers'
MODIFIER_RECIPE_TARGET = ROOT / 'tconstruct/content/data/tconstruct/recipes/tools/modifiers'


def copy_modifier_recipes():
    """The modifier and incremental-modifier station recipes, byte for byte. Other recipe types in that
    folder (salvage, swappable, ...) are separate recipe kinds and are not imported yet."""
    if MODIFIER_RECIPE_TARGET.exists():
        shutil.rmtree(MODIFIER_RECIPE_TARGET)
    count = 0
    for path in sorted(MODIFIER_RECIPE_SOURCE.rglob('*.json')):
        if json.loads(path.read_text(encoding='utf-8')).get('type') not in ('tconstruct:modifier', 'tconstruct:incremental_modifier', 'tconstruct:swappable_modifier', 'tconstruct:modifier_salvage', 'tconstruct:remove_modifier', 'tconstruct:multilevel_modifier', 'tconstruct:multilevel_incremental_modifier', 'tconstruct:overslime_modifier'):
            continue
        destination = MODIFIER_RECIPE_TARGET / path.relative_to(MODIFIER_RECIPE_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    return count


PART_RECIPE_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes/tools/parts/builder'
PART_RECIPE_TARGET = ROOT / 'tconstruct/content/data/tconstruct/recipes/tools/parts/builder'


RECYCLE_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes/tools/recycling'
RECYCLE_TARGET = ROOT / 'tconstruct/content/data/tconstruct/recipes/tools/recycling'


def copy_recycling_recipes():
    """The part builder's tool recycling recipes (a tool breaks into its parts), byte for byte."""
    if RECYCLE_TARGET.exists():
        shutil.rmtree(RECYCLE_TARGET)
    count = 0
    for path in sorted(RECYCLE_SOURCE.rglob('*.json')):
        if json.loads(path.read_text(encoding='utf-8')).get('type') != 'tconstruct:part_builder_tool_recycling':
            continue
        destination = RECYCLE_TARGET / path.relative_to(RECYCLE_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    return count


def copy_part_builder_recipes():
    """The part builder recipes that make tool parts from materials, byte for byte. The sand and
    red sand cast recipes belong to the smeltery and come with it."""
    if PART_RECIPE_TARGET.exists():
        shutil.rmtree(PART_RECIPE_TARGET)
    count = 0
    for path in sorted(PART_RECIPE_SOURCE.rglob('*.json')):
        if json.loads(path.read_text(encoding='utf-8')).get('type') != 'tconstruct:part_builder':
            continue
        destination = PART_RECIPE_TARGET / path.relative_to(PART_RECIPE_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    return count


DAMAGE_TAG_SOURCE = TAG_SOURCE / 'damage_type'
DAMAGE_TAG_TARGET = ROOT / 'tconstruct/content/data/tconstruct/tinkering/tags/damage_types'
VANILLA_DAMAGE_TAGS = ROOT / '.local/base-26.3/data/minecraft/tags/damage_type'


def copy_damage_tags():
    """Damage-type tags (protection/*, is_melee...). The host has no damage-type tag capture, so the mod reads
    them from its own folder, like the modifier tags."""
    count = 0
    for path in sorted(DAMAGE_TAG_SOURCE.rglob('*.json')):
        destination = DAMAGE_TAG_TARGET / path.relative_to(DAMAGE_TAG_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(json.dumps(json.loads(path.read_text(encoding='utf-8')), indent=2) + '\n', encoding='utf-8', newline='\n')
        count += 1
    return count


def write_damage_vanilla():
    """damage_vanilla.rs: the vanilla damage-type tags the shipped tags and protection modules name, flattened.

    Read from the pinned vanilla data, so a tag is as the game version defines it. The bypass tags are what
    mantle:can_protect (not bypassing invulnerability, as vanilla's enchantment protection) looks at."""
    # is_freezing is the source frostshield's damage_to_capacity names.
    wanted = {'bypasses_invulnerability', 'bypasses_enchantments', 'is_freezing'}
    for path in DAMAGE_TAG_SOURCE.rglob('*.json'):
        for value in json.loads(path.read_text(encoding='utf-8')).get('values', []):
            text = value if isinstance(value, str) else value.get('id', '')
            if text.startswith('#minecraft:'):
                wanted.add(text[len('#minecraft:'):])

    def flatten(name):
        data = json.loads((VANILLA_DAMAGE_TAGS / f'{name}.json').read_text(encoding='utf-8'))
        members = []
        for value in data.get('values', []):
            text = value if isinstance(value, str) else value['id']
            if text.startswith('#minecraft:'):
                members += [m for m in flatten(text[len('#minecraft:'):]) if m not in members]
            elif text not in members:
                members.append(text)
        return members

    lines = ['//! Vanilla damage-type tags the Tinkers damage tags and protection modules name, flattened.',
             '//! Generated from the pinned vanilla data by tools/import_original_data.py; do not edit by hand.', '',
             'pub(crate) const TAGS: &[(&str, &[&str])] = &[']
    for name in sorted(wanted):
        lines.append(f'    ("{name}", &[' + ', '.join(f'"{m}"' for m in flatten(name)) + ']),')
    lines.append('];')
    (ROOT / 'tconstruct/mixins/tconstruct/api/src/damage_vanilla.rs').write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')


def copy_tags():
    """Item, block and entity-type tags. Forge's tag namespace does not exist in this host, so a
    required reference to a forge tag becomes an optional one (as the original already does for
    other mods' entries); everything else is kept in order, unchanged."""
    if TAG_TARGET.exists():
        shutil.rmtree(TAG_TARGET)
    count = 0
    for source_name, target_name in TAG_FOLDER_NAMES.items():
        for path in sorted((TAG_SOURCE / source_name).rglob('*.json')):
            data = json.loads(path.read_text(encoding='utf-8'))
            values = []
            for value in data.get('values', []):
                text = value if isinstance(value, str) else value.get('id', '')
                if isinstance(value, str) and text.lstrip('#').startswith('forge:'):
                    value = {'id': text, 'required': False}
                # This port registers no Tinkers entity types yet; a tag naming one would make the whole
                # entity-type tag capture unusable for PlutoLib (an unknown member), so it waits for its entity.
                if source_name == 'entity_types' and isinstance(value, str) and text.startswith('tconstruct:'):
                    continue
                values.append(value)
            data['values'] = values
            destination = TAG_TARGET / target_name / path.relative_to(TAG_SOURCE / source_name)
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(json.dumps(data, indent=2) + '\n', encoding='utf-8', newline='\n')
            count += 1
    return count


ASSET_SOURCE = ROOT / '.local/tconstruct-source/src/main/resources/assets/tconstruct'
ASSET_TARGET = ROOT / 'tconstruct/content/assets/tconstruct'
BASE_ASSETS = ROOT / '.local/base-26.3/assets/minecraft'
HORIZON_DECLARATIONS = ROOT / 'tconstruct/content/data/tconstruct/horizon'
# Loaders that only retexture an otherwise ordinary model: dropped, the model keeps its shape.
PLAIN_LOADERS = {'mantle:retextured'}


def qualify(reference):
    return reference if ':' in reference else f'minecraft:{reference}'


def copy_client_assets():
    """Models, textures, item definitions and blockstates of the registered content that needs no per-material
    rendering, converted from the 1.20 resource-pack layout to the pinned 26.x one (SlimeKnights, MIT; see LICENSE).

    Only ids the content mod declares are imported, and only the models and textures they reach through
    parent and texture references. An item whose model needs the original's own loader (tools, tool parts,
    armor: tconstruct:tool / material loaders) is left out and returned in the skipped list. 26.x changes: every
    item needs items/<id>.json naming its model; references are written with their namespace; loader keys
    that only retexture are dropped; a block declaration copies its template's states (no `facing`), so the
    original's north variant becomes its single variant.
    """
    for folder in ('models', 'textures', 'items', 'blockstates'):
        shutil.rmtree(ASSET_TARGET / folder, ignore_errors=True)
    written = {'models': set(), 'textures': set()}
    skipped = []

    def read(path):
        return json.loads(path.read_text(encoding='utf-8'))

    def write(relative, data):
        destination = ASSET_TARGET / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(json.dumps(data, indent=2) + '\n', encoding='utf-8', newline='\n')

    def import_texture(reference):
        reference = qualify(reference)
        namespace, path = reference.split(':', 1)
        if namespace == 'minecraft':
            if not (BASE_ASSETS / 'textures' / f'{path}.png').exists():
                raise SystemExit(f'texture {reference} is not in the pinned base assets')
            return reference
        if namespace != 'tconstruct':
            raise SystemExit(f'texture {reference} belongs to another namespace')
        source = ASSET_SOURCE / 'textures' / f'{path}.png'
        if not source.exists():
            raise SystemExit(f'original texture missing: {reference}')
        if path not in written['textures']:
            written['textures'].add(path)
            destination = ASSET_TARGET / 'textures' / f'{path}.png'
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)
            meta = source.parent / f'{source.name}.mcmeta'
            if meta.exists():
                shutil.copyfile(meta, destination.parent / meta.name)
        return reference

    def import_model(reference):
        reference = qualify(reference)
        namespace, path = reference.split(':', 1)
        if namespace == 'minecraft':
            if not (BASE_ASSETS / 'models' / f'{path}.json').exists():
                raise SystemExit(f'model {reference} is not in the pinned base assets')
            return reference
        if namespace != 'tconstruct':
            raise SystemExit(f'model {reference} belongs to another namespace')
        if path in written['models']:
            return reference
        written['models'].add(path)
        converted = {}
        for key, value in read(ASSET_SOURCE / 'models' / f'{path}.json').items():
            if key in ('loader', 'retextured', '__comment'):
                continue
            if key == 'parent':
                value = import_model(value)
            elif key == 'textures':
                value = {name: texture if texture.startswith('#') else import_texture(texture) for name, texture in value.items()}
            elif key == 'elements':
                value = [{k: v for k, v in element.items() if k != '__comment'} for element in value]
            converted[key] = value
        write(f'models/{path}.json', converted)
        return reference

    def needs_loader(reference):
        namespace, path = qualify(reference).split(':', 1)
        if namespace != 'tconstruct':
            return False
        model = read(ASSET_SOURCE / 'models' / f'{path}.json')
        return model.get('loader') not in (None, *PLAIN_LOADERS) or ('parent' in model and needs_loader(model['parent']))

    shutil.copyfile(ROOT / 'tconstruct/LICENSE', ASSET_TARGET / 'license')
    for name in sorted(path.stem for path in (HORIZON_DECLARATIONS / 'block').glob('*.json')):
        variants = read(ASSET_SOURCE / 'blockstates' / f'{name}.json').get('variants', {})
        if sorted(variants) != ['facing=east', 'facing=north', 'facing=south', 'facing=west']:
            raise SystemExit(f'block {name}: unexpected blockstate shape')
        variant = dict(variants['facing=north'])
        if needs_loader(variant['model']):
            skipped.append(name)
            continue
        variant['model'] = import_model(variant['model'])
        write(f'blockstates/{name}.json', {'variants': {'': variant}})
        item_model = (ASSET_SOURCE / 'models/item' / f'{name}.json').exists()
        write(f'items/{name}.json', {'model': {'type': 'minecraft:model', 'model': import_model(f'tconstruct:item/{name}') if item_model else variant['model']}})
    for name in sorted(path.stem for path in (HORIZON_DECLARATIONS / 'item').glob('*.json')):
        if not (ASSET_SOURCE / 'models/item' / f'{name}.json').exists() or needs_loader(f'tconstruct:item/{name}'):
            skipped.append(name)
            continue
        write(f'items/{name}.json', {'model': {'type': 'minecraft:model', 'model': import_model(f'tconstruct:item/{name}')}})
    return len(written['models']), len(written['textures']), skipped

GENERATED_ASSETS = ROOT / '.local/tconstruct-source/src/generated/resources/assets/tconstruct'
MATERIAL_INFO = GENERATED_ASSETS / 'tinkering/materials'
MATERIAL_DEFINITIONS = ROOT / 'tconstruct/content/data/tconstruct/tinkering/materials/definition'
# Pixel offset (+x right, +y down) the original's material loader applies; a face of the unit element moved the same way.
FLAT_FACES = {'north': {'uv': [16, 0, 0, 16], 'texture': '#layer0'}, 'south': {'uv': [0, 0, 16, 16], 'texture': '#layer0'}}


def material_sprites():
    """Per shipped material: the redirect chain of sprite names and the fallback names, from the original's render info."""
    info = {}
    for path in MATERIAL_INFO.rglob('*.json'):
        info['_'.join(path.relative_to(MATERIAL_INFO).with_suffix('').parts)] = json.loads(path.read_text(encoding='utf-8'))
    result = {}
    for material in sorted(path.stem for path in MATERIAL_DEFINITIONS.glob('*.json')):
        chain, fallbacks, name = [], [], material
        while name is not None and name not in chain:
            chain.append(name)
            data = info.get(name, {})
            fallbacks += [fallback for fallback in data.get('fallbacks', []) if fallback not in fallbacks]
            target = data.get('texture')
            name = target.split(':', 1)[1] if target and target.startswith('tconstruct:') else None
        result[material] = (chain, fallbacks)
    return result


class MaterialSprites:
    """Collects the models and sprites of material-dependent item layers and writes them once, byte for byte."""

    def __init__(self):
        self.sprites = material_sprites()
        self.models = {}
        self.textures = {}

    @staticmethod
    def source_of(layer, suffix):
        for root in (GENERATED_ASSETS, ASSET_SOURCE):
            if (root / 'textures' / f'{layer}{suffix}.png').is_file():
                return root / 'textures' / f'{layer}{suffix}.png'
        return None

    def model_for(self, folder, parent, layer, suffix, offset=(0, 0), large=None):
        """The model reference for a layer sprite, or None when that sprite does not exist. Models are shared by
        everything in `folder` that uses the same sprite file. `offset` is the material model's pixel shift;
        `large` is the tool's large_offset when the sprite is a 32x32 large variant."""
        source = self.source_of(layer, suffix)
        if source is None:
            return None
        name = f'item/{folder}/{layer.removeprefix("item/tool/")}{suffix}'
        self.textures[f'{layer}{suffix}'] = source
        model = {'parent': parent, 'textures': {'layer0': f'tconstruct:{layer}{suffix}'}}
        if large is not None:
            # The original draws the large quads twice as wide and tall, shifted by ((x - 8) / 32, (-y - 8) / 32) blocks.
            x, y = (large[0] - 8) / 2, (-large[1] - 8) / 2
            model['elements'] = [{'from': [x, y, 7.5], 'to': [x + 32, y + 32, 8.5], 'faces': FLAT_FACES}]
        elif tuple(offset) != (0, 0):
            dx, dy = offset
            model['elements'] = [{'from': [dx, -dy, 7.5], 'to': [16 + dx, 16 - dy, 8.5], 'faces': FLAT_FACES}]
        self.models[name] = model
        return {'type': 'minecraft:model', 'model': f'tconstruct:{name}'}

    def select(self, folder, parent, layer, offset=(0, 0), large=None):
        """(fallback model, cases) for a select on custom_model_data string 0 (a material id path) over the
        shipped materials' sprites; the fallback is the base sprite."""
        fallback = self.model_for(folder, parent, layer, '', offset, large)
        if fallback is None:
            raise SystemExit(f'base sprite {layer} is missing')
        cases = []
        for material, (chain, fallbacks) in self.sprites.items():
            candidates = [f'_tconstruct_{name}' for name in chain] + [f'_{name}' for name in fallbacks]
            model = next((m for m in (self.model_for(folder, parent, layer, suffix, offset, large) for suffix in candidates) if m), None)
            if model:
                cases.append({'when': material, 'model': model})
        return fallback, cases

    def write(self, models=None):
        for name, model in {**self.models, **(models or {})}.items():
            write_json(ASSET_TARGET / 'models' / f'{name}.json', model)
        for name, source in self.textures.items():
            destination = ASSET_TARGET / 'textures' / f'{name}.png'
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)


def write_json(destination, data, compact=False):
    destination.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(data, separators=(',', ':')) if compact else json.dumps(data, indent=2)
    destination.write_text(text + '\n', encoding='utf-8', newline='\n')


def original_model(path):
    return json.loads(path.read_text(encoding='utf-8'))


def material_select(fallback, cases, index):
    return {'type': 'minecraft:select', 'property': 'minecraft:custom_model_data', 'index': index, 'cases': cases, 'fallback': fallback} if cases else fallback


def copy_part_assets():
    """Item definitions, models and sprites of the tool parts: one `select` on custom_model_data string 0 (the
    material id path), a case per shipped material that has a sprite, the grayscale base sprite as fallback.

    Sprite choice follows the original's material render info: the pre-baked `<layer>_tconstruct_<material>.png`
    of the material or of its `texture` redirect target, else the hand-made `<layer>_<fallback>.png` for its
    first fallback name that exists, else the base sprite. Files are copied byte for byte (SlimeKnights, MIT).
    The original's `offset` becomes a flat element moved by the same pixels. Returns (part names, model count, texture count).
    """
    collected = MaterialSprites()
    parts = []
    for path in sorted((HORIZON_DECLARATIONS / 'item').glob('*.json')):
        original = GENERATED_ASSETS / 'models/item' / f'{path.stem}.json'
        if not original.exists() or original_model(original).get('loader') != 'tconstruct:material':
            continue
        data = original_model(original)
        layer = data['textures']['texture'].split(':', 1)[1]
        fallback, cases = collected.select(f'part/{path.stem}', 'minecraft:item/generated', layer, data.get('offset', [0, 0]))
        write_json(ASSET_TARGET / 'items' / f'{path.stem}.json', {'model': material_select(fallback, cases, 0)})
        parts.append(path.stem)
    collected.write()
    return parts, len(collected.models), len(collected.textures)


ARMOR_PREFIXES = ('plate_', 'slime_', 'travelers_')
FORGE_PARENTS = {'forge:item/default': 'minecraft:item/generated', 'forge:item/default-tool': 'minecraft:item/handheld'}
MODEL_SOURCE = ASSET_SOURCE / 'models'
MODIFIER_SPRITES = GENERATED_ASSETS / 'tinkering/modifiers/sprites'
# custom_model_data.flags: what the tool item definitions read besides the material strings (see model_flags.rs).
BLOCKING_FLAG, SPEAR_FLAG, ARROW_FLAG, FIREWORK_FLAG, OVERLAY_BASE = 0, 1, 2, 3, 4
# Elapsed use ticks per unit of the original's `charge` property (its default draw time, KEY_DRAWTIME, is 20 ticks).
CHARGE_SCALE = 0.05
FLAGS_TARGET = ROOT / 'tconstruct/mixins/tconstruct/api/src/model_flags.rs'


def original_model_json(reference):
    """The original's model JSON: the generated resources (broken/blocking/pulling states) before the hand-written ones."""
    path = reference.split(':', 1)[1]
    for root in (GENERATED_ASSETS, ASSET_SOURCE):
        if (root / 'models' / f'{path}.json').is_file():
            return original_model(root / 'models' / f'{path}.json')
    raise SystemExit(f'original model missing: {reference}')


def resolve_state(reference):
    """(tool model spec, display overrides, gui_light) of a tool state model. A state is either a full tool model
    (loader tconstruct:tool) or a plain model that only adds display transforms to another state."""
    data = original_model_json(reference)
    if data.get('loader') == 'tconstruct:tool':
        return data, {}, None
    spec, display, light = resolve_state(data['parent'])
    own = {key: value for key, value in data.get('display', {}).items() if ':' not in key}
    return spec, {**display, **own}, data.get('gui_light', light)


def overlay_textures(spec, large):
    """{modifier id: [texture layers]} the tool model draws for a modifier, from its modifier model maps (the first
    map that names a modifier wins, an empty entry removes it). A large tool draws only `texture_large`. Fluid, tank,
    potion, dyed and trim models are dynamic (tint, fill level) and left out."""
    merged = {}
    for option in reversed(spec.get('modifier_maps', [])):
        path = MODIFIER_SPRITES / f'{option.split(":", 1)[1]}.json'
        if path.is_file():
            merged.update(original_model(path).get('modifiers', {}))
    result = {}
    for modifier, value in merged.items():
        layers = []
        for entry in value if isinstance(value, list) else [value]:
            entry = {'texture': entry} if isinstance(entry, str) else entry
            if entry.get('type') not in (None, 'tconstruct:normal'):
                continue
            texture = entry.get('texture_large' if large else 'texture')
            if texture:
                layers.append(texture.split(':', 1)[1])
        if layers:
            result[modifier] = layers
    return result


def state_overrides(overrides):
    """[(predicate, model)] of a tool model's overrides; as in vanilla 1.20 the last matching entry wins."""
    known = {'tconstruct:broken', 'tconstruct:cast', 'tconstruct:ammo', 'tconstruct:charging', 'tconstruct:charge'}
    for override in overrides:
        if not set(override['predicate']) <= known:
            raise SystemExit(f'unknown tool model predicate: {override["predicate"]}')
    return [(override['predicate'], override['model']) for override in overrides]


def state_winner(overrides, root, state):
    chosen = root
    values = {'tconstruct:broken': state['broken'], 'tconstruct:cast': state['cast'], 'tconstruct:ammo': state['ammo'],
              'tconstruct:charging': state['charging'], 'tconstruct:charge': state['charge']}
    for predicate, model in overrides:
        if all(values[key] >= threshold for key, threshold in predicate.items()):
            chosen = model
    return chosen


def flag_condition(index, on_true, on_false):
    if on_true == on_false:
        return on_true
    return {'type': 'minecraft:condition', 'property': 'minecraft:custom_model_data', 'index': index, 'on_true': on_true, 'on_false': on_false}


def property_condition(name, on_true, on_false):
    if on_true == on_false:
        return on_true
    return {'type': 'minecraft:condition', 'property': name, 'on_true': on_true, 'on_false': on_false}


def state_tree(overrides, root, leaf, gui):
    """The item model tree over a tool's override states. Variables, outermost first: broken (`minecraft:broken`,
    the host's damage >= max_damage - 1, which is the stack of a broken tool), fishing rod cast, loaded ammo
    (flags), then use: `using_item`, the use animation (flags: blocking, spear, otherwise drawn like a bow) and the
    elapsed use ticks (`use_duration`, scaled to the original's charge). Equal subtrees collapse. In the GUI nobody
    is using or casting, so only broken and ammo apply."""
    predicates = {key for predicate, _ in overrides for key in predicate}
    charges = sorted({value for predicate, _ in overrides for key, value in predicate.items() if key == 'tconstruct:charge' and value > 0})

    def used(state, rest, charging):
        fallback = build({**state, 'charging': charging, 'charge': 0}, rest)
        entries = []
        for threshold in charges:
            model = build({**state, 'charging': charging, 'charge': threshold}, rest)
            if model != (entries[-1]['model'] if entries else fallback):
                entries.append({'threshold': threshold, 'model': model})
        if not entries:
            return fallback
        return {'type': 'minecraft:range_dispatch', 'property': 'minecraft:use_duration', 'scale': CHARGE_SCALE, 'entries': entries, 'fallback': fallback}

    def build(state, steps):
        if not steps:
            return leaf(state_winner(overrides, root, state))
        step, rest = steps[0], steps[1:]
        if step == 'broken':
            return property_condition('minecraft:broken', build({**state, 'broken': 1}, rest), build({**state, 'broken': 0}, rest))
        if step == 'cast':
            return property_condition('minecraft:fishing_rod/cast', build({**state, 'cast': 1}, rest), build({**state, 'cast': 0}, rest))
        if step == 'ammo':
            plain = flag_condition(ARROW_FLAG, build({**state, 'ammo': 1}, rest), build({**state, 'ammo': 0}, rest))
            return flag_condition(FIREWORK_FLAG, build({**state, 'ammo': 2}, rest), plain)
        # use: the charge entries read the elapsed use ticks, so they sit below `using_item`
        pull = flag_condition(SPEAR_FLAG, used(state, rest, 1.75), used(state, rest, 1))
        return property_condition('minecraft:using_item', flag_condition(BLOCKING_FLAG, used(state, rest, 2), pull), build({**state, 'charging': 0, 'charge': 0}, rest))

    if gui:
        steps = [step for step, key in (('broken', 'tconstruct:broken'), ('ammo', 'tconstruct:ammo')) if key in predicates]
    else:
        steps = [step for step, keys in (('broken', {'tconstruct:broken'}), ('cast', {'tconstruct:cast'}), ('ammo', {'tconstruct:ammo'}),
                                         ('use', {'tconstruct:charging', 'tconstruct:charge'})) if keys & predicates]
    return build({'broken': 0, 'cast': 0, 'ammo': 0, 'charging': 0, 'charge': 0}, steps)


def copy_tool_assets():
    """Item definitions for the registered tools with material layers (armor excluded, shields included): the
    original's tool model as one item model tree.

    A leaf is a `composite` of one layer per original model part, in the original's draw order, each a `select` on
    the custom_model_data string at the part's material slot index (the slot of the tool definition's part_stats),
    then one overlay per modifier the tool model has a sprite for, each a `condition` on a custom_model_data flag
    (model_flags.rs: the modifier's position in the table the importer writes). A tool without parts (staffs, flint
    and brick) draws its single sprite. The original's display parents are kept where they carry transforms
    (`tconstruct:thrown`, a Forge-only context, is dropped; Forge's default parents become the vanilla generated/handheld ones).

    The override states of the original's tool model (broken, blocking, drawing, throwing, ammo, cast) select among
    leaves through `state_tree`; a state's own textures (head_broken, bowstring_1) and display transforms apply.
    A `large` tool draws its 32x32 sprites in every context but the GUI (as the original's BakedToolModel does), as flat
    quads twice the size, shifted by large_offset. Dynamic layers (fluid, tank fill, potion, dye, trim) are left out.
    Returns (tool names, model count, texture count, tools left out).
    """
    collected = MaterialSprites()
    extra = {}
    states = {}

    def parent_of(reference):
        if reference in FORGE_PARENTS:
            return FORGE_PARENTS[reference]
        namespace, path = reference.split(':', 1) if ':' in reference else ('minecraft', reference)
        if namespace != 'tconstruct':
            return f'{namespace}:{path}'
        data = original_model_json(reference)
        display = {key: value for key, value in data.get('display', {}).items() if ':' not in key}
        if not display:
            return parent_of(data['parent'])
        converted = {'parent': parent_of(data['parent']), 'display': display}
        if 'gui_light' in data:
            converted['gui_light'] = data['gui_light']
        extra[path] = converted
        return reference

    def state_parent(spec, display, light):
        base = parent_of(spec['parent'])
        if not display:
            return base
        key = json.dumps([base, display, light], sort_keys=True)
        if key not in states:
            states[key] = f'item/tool_state/{len(states) + 1:02d}'
            extra[states[key]] = {'parent': base, 'display': display, **({'gui_light': light} if light else {})}
        return f'tconstruct:{states[key]}'

    tools, left_out, specs = [], [], {}
    for path in sorted((TARGET / 'tool_definitions').glob('*.json')):
        name = path.stem
        original = MODEL_SOURCE / 'item' / f'{name}.json'
        if (name.startswith(ARMOR_PREFIXES) and not name.endswith('_shield')) or not original.exists() or original_model(original).get('loader') != 'tconstruct:tool':
            left_out.append(name)
            continue
        modules = original_model(path)['modules']
        # Material slots: the parts of part_stats, or the stat types of material_stats (tools made of materials alone).
        slots = sum(len(m.get('parts') or m.get('stat_types') or []) for m in modules if m['type'] in ('tconstruct:part_stats', 'tconstruct:material_stats'))
        specs[name] = (original_model(original), slots)

    # The flag table: every modifier any tool state draws a sprite for, in id order.
    table = sorted({modifier for name, (data, _) in specs.items()
                    for ref in [f'tconstruct:item/{name}'] + [override['model'] for override in data.get('overrides', [])]
                    for large in (False, True) for modifier in overlay_textures(resolve_state(ref)[0], large)})

    def leaf(name, ref, large):
        spec, display, light = resolve_state(ref)
        slots = specs[name][1]
        parent = state_parent(spec, display, light)
        folder = 'tool_layer/' + parent.split(':', 1)[1].replace('/', '_')
        offset = spec.get('large_offset', [0, 0]) if large else None
        prefix = 'large_' if large else ''
        models = []
        for part in spec.get('parts') or [{'name': 'tool'}]:
            layer = spec['textures'][prefix + part['name']].split(':', 1)[1]
            if 'index' in part:
                if part['index'] >= slots:
                    raise SystemExit(f'tool {name}: part {part["name"]} index {part["index"]} is outside the {slots} material slots')
                fallback, cases = collected.select(folder, parent, layer, (0, 0), offset)
                models.append(material_select(fallback, cases, part['index']))
            else:
                model = collected.model_for(folder, parent, layer, '', (0, 0), offset)
                if model is None:
                    raise SystemExit(f'tool {name}: sprite {layer} is missing')
                models.append(model)
        for modifier, layers in overlay_textures(spec, large).items():
            for layer in layers:
                model = collected.model_for(folder, parent, layer, '', (0, 0), offset)
                if model is None:
                    raise SystemExit(f'tool {name}: modifier sprite {layer} is missing')
                models.append({'type': 'minecraft:condition', 'property': 'minecraft:custom_model_data', 'index': OVERLAY_BASE + table.index(modifier),
                               'on_true': model, 'on_false': {'type': 'minecraft:empty'}})
        return models[0] if len(models) == 1 else {'type': 'minecraft:composite', 'models': models}

    for name, (data, _) in specs.items():
        overrides = state_overrides(data.get('overrides', []))
        root = f'tconstruct:item/{name}'
        full = state_tree(overrides, root, lambda ref: leaf(name, ref, data.get('large', False)), False)
        if data.get('large'):
            gui = state_tree(overrides, root, lambda ref: leaf(name, ref, False), True)
            full = {'type': 'minecraft:select', 'property': 'minecraft:display_context', 'cases': [{'when': 'gui', 'model': gui}], 'fallback': full}
        write_json(ASSET_TARGET / 'items' / f'{name}.json', {'model': full}, compact=True)
        tools.append(name)
    collected.write(extra)
    write_model_flags(table)
    return tools, len(collected.models) + len(extra), len(collected.textures), left_out


def write_model_flags(table):
    lines = ['//! Positions in `custom_model_data.flags` that the imported tool item definitions read (SlimeKnights, MIT; see LICENSE).',
             '//! Generated from the original tool models by tools/import_original_data.py; do not edit.',
             '//!',
             '//! The first four are use-time facts the tool item definitions dispatch on while the tool is held in use',
             '//! (`using_item` plus the use animation, the crossbow ammo state). `OVERLAYS` lists the modifiers some tool model draws a',
             '//! sprite for; flag `OVERLAY_BASE + i` is set while the tool has `OVERLAYS[i]` among its crafted modifiers.',
             '#![allow(dead_code)]', '',
             f'pub(crate) const BLOCKING: usize = {BLOCKING_FLAG};', f'pub(crate) const SPEAR: usize = {SPEAR_FLAG};',
             f'pub(crate) const ARROW: usize = {ARROW_FLAG};', f'pub(crate) const FIREWORK: usize = {FIREWORK_FLAG};',
             f'pub(crate) const OVERLAY_BASE: usize = {OVERLAY_BASE};', '', 'pub(crate) const OVERLAYS: &[&str] = &[']
    lines += [f'    "{modifier}",' for modifier in table] + ['];']
    FLAGS_TARGET.write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')


LANG_SOURCE = ROOT / '.local/tconstruct-source/src/main/resources/assets/tconstruct/lang'
LANG_TARGET = ROOT / 'tconstruct/content/assets/tconstruct/lang'
LANGUAGES = ['en_us', 'ja_jp']


def copy_lang():
    """The original's translations for the languages this port ships (modifier, block, item,
    material and gui names): the content mod's own lang files, read by the game and by PlutoLib's I18n."""
    if LANG_TARGET.exists():
        shutil.rmtree(LANG_TARGET)
    LANG_TARGET.mkdir(parents=True)
    for language in LANGUAGES:
        # Only the line endings are normalized (the original checkout may be CRLF); the text is the original's.
        data = (LANG_SOURCE / f'{language}.json').read_bytes().replace(b'\r\n', b'\n')
        (LANG_TARGET / f'{language}.json').write_bytes(data)
    return len(LANGUAGES)


def main():
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    head = subprocess.run(['git', '-C', str(ROOT / '.local/tconstruct-source'), 'rev-parse', 'HEAD'],
                          capture_output=True, text=True, check=True).stdout.strip()
    expected = pins.get('tconstruct_revision') or pins.get('java_revision') or head
    if head != expected:
        raise SystemExit(f'original checkout {head} differs from the pinned {expected}')
    skipped_materials = set()
    for path in sorted((SOURCE / 'materials/definition').glob('*.json')):
        if 'condition' in json.loads(path.read_text(encoding='utf-8')) and external(json.loads(path.read_text(encoding='utf-8'))['condition']):
            skipped_materials.add(path.stem)
    if TARGET.exists():
        shutil.rmtree(TARGET)
    copied = 0
    for folder in FOLDERS + TAG_FOLDERS:
        for path in sorted((SOURCE / folder).rglob('*.json')):
            relative = path.relative_to(SOURCE)
            if relative.parts[0] == 'tool_definitions' and path.stem in EXCLUDED_TOOLS:
                continue
            if relative.parts[0] == 'materials' and path.stem in skipped_materials:
                continue
            destination = TARGET / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)
            copied += 1
    types = sorted({module['type'] for path in (TARGET / 'modifiers').glob('*.json')
                    for module in json.loads(path.read_text(encoding='utf-8')).get('modules', []) or []})
    lines = ['//! Module type IDs the original ships in its modifier data (SlimeKnights, MIT; see LICENSE).',
             '//! A listed module that Tinkers has not ported is recorded as unbound, never evaluated; an ID',
             '//! outside this list is an unknown module and rejects the whole modifier definition.',
             '//! Generated from the pinned original data by tools/import_original_data.py.', '',
             'pub(crate) const KNOWN: &[&str] = &[']
    lines += [f'    "{kind}",' for kind in types] + ['];']
    (ROOT / 'tconstruct/mixins/tconstruct/api/src/modifier_module_types.rs').write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
    write_creative_parts(skipped_materials)
    write_creative_tools()
    tags = copy_tags() + copy_damage_tags()
    write_damage_vanilla()
    languages = copy_lang()
    models, textures, unrendered = copy_client_assets()
    part_names, part_models, part_textures = copy_part_assets()
    tool_names, tool_models, tool_textures, tools_left_out = copy_tool_assets()
    models, textures = len(list((ASSET_TARGET / 'models').rglob('*.json'))), len(list((ASSET_TARGET / 'textures').rglob('*.png')))
    unrendered = [name for name in unrendered if name not in part_names + tool_names]
    recipes = copy_material_recipes(skipped_materials) + copy_modifier_recipes() + copy_part_builder_recipes() + copy_recycling_recipes()
    print(f'copied {copied} files, {tags} tags, {languages} language files and {recipes} material recipes from {head}; {models} models and {textures} textures (left out, need the original tool loader: {len(unrendered)}; tools with no layered item definition: {tools_left_out}); {len(types)} modifier module types; excluded materials: {sorted(skipped_materials)}')


if __name__ == '__main__':
    sys.exit(main())
