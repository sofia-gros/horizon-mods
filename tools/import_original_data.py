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
    wanted = {'bypasses_invulnerability', 'bypasses_enchantments'}
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

    shutil.copyfile(ROOT / 'tconstruct/LICENSE', ASSET_TARGET / 'LICENSE')
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
    recipes = copy_material_recipes(skipped_materials) + copy_modifier_recipes() + copy_part_builder_recipes() + copy_recycling_recipes()
    print(f'copied {copied} files, {tags} tags, {languages} language files and {recipes} material recipes from {head}; {models} models and {textures} textures (left out, need the original tool loader: {len(unrendered)}); {len(types)} modifier module types; excluded materials: {sorted(skipped_materials)}')


if __name__ == '__main__':
    sys.exit(main())
