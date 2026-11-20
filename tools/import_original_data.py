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


SEVERING_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes/tools/severing'
SEVERING_TARGET = ROOT / 'tconstruct/content/data/tconstruct/recipes/tools/severing'


def copy_severing_recipes():
    """The severing recipes that turn a mob type into an item, byte for byte. The recipes whose output depends on the entity's own state
    (ageable, mooshroom, player, sheep and snow golem) are left out: the loot event reports no such state."""
    if SEVERING_TARGET.exists():
        shutil.rmtree(SEVERING_TARGET)
    count = 0
    for path in sorted(SEVERING_SOURCE.rglob('*.json')):
        if json.loads(path.read_text(encoding='utf-8')).get('type') != 'tconstruct:severing':
            continue
        destination = SEVERING_TARGET / path.relative_to(SEVERING_SOURCE)
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


FLUID_RECIPE_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes'
FLUID_RECIPE_TARGET = ROOT / 'tconstruct/content/data/tconstruct/recipes/fluids'
FLUID_TAG_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data'
FLUID_TAG_TARGET = ROOT / 'tconstruct/content/data/tconstruct/tinkering/fluid_tags'
# The smeltery domain's recipe types (melting, alloying, fuel, casting, molding, material fluids), including the tool and
# potion casting kinds that are copied for completeness but not decoded yet. Wherever they sit in the original's tree.
FLUID_RECIPE_TYPES = {
    'tconstruct:melting', 'tconstruct:ore_melting', 'tconstruct:damagable_melting', 'tconstruct:material_melting', 'tconstruct:melting_fuel',
    'tconstruct:entity_melting', 'tconstruct:alloy', 'tconstruct:material_fluid', 'tconstruct:molding_table',
    'tconstruct:casting_table', 'tconstruct:casting_basin', 'tconstruct:retextured_casting_basin', 'tconstruct:table_duplication',
    'tconstruct:casting_table_potion', 'tconstruct:table_filling', 'tconstruct:basin_filling',
    'tconstruct:table_casting_material', 'tconstruct:basin_casting_material', 'tconstruct:table_casting_composite', 'tconstruct:basin_casting_composite',
    'tconstruct:basin_tool_casting', 'tconstruct:table_tool_casting', 'tconstruct:basin_casting_part_swapping', 'tconstruct:table_casting_part_swapping',
    'tconstruct:casting_table_tipping', 'tconstruct:casting_table_tipped_clearing',
}


def copy_fluid_recipes():
    """The smeltery domain's recipes, byte for byte, under recipes/fluids with the original's relative path. Other-mod
    integration (the compat folder) is left out. A forge:conditional file is kept when any wrapped recipe is of a listed
    type. The sand and clay cast pattern recipes of the part builder (item_part_builder) in smeltery/casts come with them."""
    if FLUID_RECIPE_TARGET.exists():
        shutil.rmtree(FLUID_RECIPE_TARGET)
    count = 0
    for path in sorted(FLUID_RECIPE_SOURCE.rglob('*.json')):
        relative = path.relative_to(FLUID_RECIPE_SOURCE)
        if relative.parts[0] == 'compat':
            continue
        data = json.loads(path.read_text(encoding='utf-8'))
        types = [entry['recipe']['type'] for entry in data['recipes']] if data['type'] == 'forge:conditional' else [data['type']]
        if not any(kind in FLUID_RECIPE_TYPES or (kind == 'tconstruct:item_part_builder' and relative.parts[:2] == ('smeltery', 'casts')) for kind in types):
            continue
        destination = FLUID_RECIPE_TARGET / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    return count


def copy_fluid_tags():
    """The fluid tags the recipes name (forge:molten_iron, mantle:soup...), byte for byte, in the mod's own folder per
    namespace, for the recipes' FluidTags. The host's own tag folder gets the converted tconstruct ones
    (copy_fluid_host_tags): these keep the flowing variants and the forge/mantle namespaces the host does not have."""
    if FLUID_TAG_TARGET.exists():
        shutil.rmtree(FLUID_TAG_TARGET)
    count = 0
    for namespace in ('tconstruct', 'forge', 'mantle'):
        for path in sorted((FLUID_TAG_SOURCE / namespace / 'tags/fluids').rglob('*.json')):
            destination = FLUID_TAG_TARGET / namespace / path.relative_to(FLUID_TAG_SOURCE / namespace / 'tags/fluids')
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)
            count += 1
    return count


FLUID_JAVA = ROOT / '.local/tconstruct-source/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java'
FLUID_TEXTURES = ROOT / '.local/tconstruct-source/src/generated/resources/assets/tconstruct/mantle/fluid_texture'
FLUID_DEFS_TARGET = ROOT / 'tconstruct/mixins/tconstruct/api/src/fluid_defs.rs'
FLUID_HOST_TAG_TARGET = ROOT / 'tconstruct/content/data/tconstruct/tags/fluid'
# FluidType.Properties defaults (Forge): water's numbers. The helper builders of TinkerFluids start from these.
FLUID_DEFAULT = {'temperature': 300, 'light': 0, 'density': 1000, 'viscosity': 1000}
FLUID_BUILDERS = {
    'cool': {},
    'powder': {},
    'slime': {'density': 1600, 'viscosity': 1600},
    'hot': {'density': 2000, 'viscosity': 10000, 'temperature': 1000},
}
FLUID_SETTERS = {'temperature': 'temperature', 'lightLevel': 'light', 'density': 'density', 'viscosity': 'viscosity'}


def fluid_properties(expression):
    """Evaluate one FluidType.Properties chain of TinkerFluids (`hot("x").temperature(900).lightLevel(6)`): the helper
    builder's defaults, then each setter in order. Anything this does not know stops the import."""
    values = dict(FLUID_DEFAULT)
    base = re.search(r'\b(cool|powder|slime|hot)\(("[a-z_]*"\s*)?\)', expression)
    if not base:
        raise SystemExit(f'fluid properties without a known builder: {expression}')
    values.update(FLUID_BUILDERS[base.group(1)])
    for name, argument in re.findall(r'\.(\w+)\((-?\d+)\)', expression[base.end():]):
        if name not in FLUID_SETTERS:
            raise SystemExit(f'unknown fluid property setter {name} in {expression}')
        values[FLUID_SETTERS[name]] = int(argument)
    return values


def fluid_type_expression(name, rest):
    """The FluidType.Properties expression of one registration line: the second argument of registerSlime(name, props),
    or the argument of .type(...) / .invertedType(...) up to the call that follows it."""
    if rest.startswith(', '):
        return rest[2:rest.index(').bucket(') if ').bucket(' in rest else len(rest)]
    found = re.search(r'\.(?:type|invertedType)\((.*?)\)\.(?:bucket|block|burningBlock|commonTag|unplacable|tickRate|flowing|invertedFlowing)\(', rest)
    if not found:
        raise SystemExit(f'fluid {name}: no type in {rest}')
    return found.group(1)


def write_fluid_defs():
    """fluid_defs.rs: the original's 72 fluids with temperature, luminance, density, viscosity and tint, in TinkerFluids order.

    Attributes are the FluidType.Properties of each registration (the `hot`/`slime`/`cool`/`powder` helper defaults, then the
    setters), the tint is the colour of the generated mantle fluid_texture (white when it has none). The names are the
    original's `fluid.tconstruct.<name>` translations, already in the lang files. Blocks, buckets and flowing fluids are not
    declared: the host has no mod fluid blocks."""
    text = FLUID_JAVA.read_text(encoding='utf-8')
    rows = []
    for line in text.splitlines():
        match = re.search(r'=\s*FLUIDS\.register\w*\("([a-z_]+)"(.*)$', line)
        if not match:
            continue
        name, rest = match.groups()
        values = fluid_properties(fluid_type_expression(name, rest))
        texture = FLUID_TEXTURES / f'{name}.json'
        color = int(json.loads(texture.read_text(encoding='utf-8')).get('color', 'FFFFFFFF'), 16) if texture.exists() else 0xFFFFFFFF
        rows.append((name, values['temperature'], values['light'], values['density'], values['viscosity'], color))
    if len(rows) != 72 or len({row[0] for row in rows}) != 72:
        raise SystemExit(f"expected the original's 72 distinct fluids, found {len(rows)}")
    lines = ["//! The original's 72 fluids (SlimeKnights TinkerFluids, MIT; see LICENSE): attributes and tint, in registration order.",
             '//! Generated from the pinned original by tools/import_original_data.py; do not edit by hand.', '',
             '/// One fluid: id path (namespace tconstruct), kelvin, luminance, density, viscosity and tint (0xAARRGGBB).',
             "pub(crate) struct FluidSpec { pub name: &'static str, pub temperature: i32, pub luminance: u8, pub density: i32, pub viscosity: i32, pub color: u32 }",
             '', 'pub(crate) const FLUIDS: &[FluidSpec] = &[']
    for name, temperature, light, density, viscosity, color in rows:
        lines.append(f'    FluidSpec {{ name: "{name}", temperature: {temperature}, luminance: {light}, density: {density}, viscosity: {viscosity}, color: 0x{color:08X} }},')
    lines.append('];')
    FLUID_DEFS_TARGET.write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
    return len(rows), {row[0] for row in rows}


def copy_fluid_host_tags(fluids):
    """The Tinkers fluid tags in the host's tag folder (data/tconstruct/tags/fluid), which PlutoLib reads through
    reload_registry("minecraft:fluid"). The fluids are registered now, so the host may read them. Compared with the
    byte-exact copy in fluid_tags (kept for the recipes): the flowing variants are left out (the host has no flowing mod
    fluids; they belong to the world blocks), and a reference to a forge or mantle tag is replaced by that tag's own entries,
    since those namespaces do not exist in this host (the tag files themselves stay in fluid_tags for the recipes). A
    reference to a tag the original does not ship (Forge's milk, the Fabric recipe-viewer tag) becomes an optional one. Any
    other member must be a registered fluid or vanilla's lava."""
    shutil.rmtree(FLUID_HOST_TAG_TARGET, ignore_errors=True)
    source = FLUID_TAG_SOURCE / 'tconstruct/tags/fluids'
    # The original's forge and mantle tags (forge:molten_iron = #tconstruct:molten_iron, mantle:soup...): their namespaces do not
    # exist in the host, so a reference to one is replaced by that tag's own entries, and an unshipped one (forge:milk) is optional.
    foreign = {}
    for namespace in ('forge', 'mantle'):
        for path in sorted((FLUID_TAG_SOURCE / namespace / 'tags/fluids').rglob('*.json')):
            name = f'{namespace}:{path.relative_to(FLUID_TAG_SOURCE / namespace / "tags/fluids").with_suffix("").as_posix()}'
            foreign[name] = json.loads(path.read_text(encoding='utf-8')).get('values', [])

    def convert(entries, origin, seen):
        values = []
        for value in entries:
            text = value if isinstance(value, str) else value['id']
            required = isinstance(value, str) or value.get('required', True)
            if text.startswith('tconstruct:flowing_'):
                continue
            if text.startswith('#tconstruct:'):
                values.append(value)
            elif text.startswith('#'):
                name = text[1:]
                if name in foreign and name not in seen:
                    values.extend(convert(foreign[name], origin, seen | {name}))
                elif name in foreign:
                    raise SystemExit(f'{origin}: foreign tag {name} contains itself')
                else:
                    values.append({'id': text, 'required': False})
                    if required and not name.startswith(('forge:', 'c:')):
                        raise SystemExit(f'{origin}: unknown tag {name}')
            elif not ((text.startswith('tconstruct:') and text.split(':')[1] in fluids) or text == 'minecraft:lava'):
                raise SystemExit(f'{origin}: member {text} is neither a Tinkers fluid nor lava')
            else:
                values.append(value)
        return values

    count = 0
    for path in sorted(source.rglob('*.json')):
        values = convert(json.loads(path.read_text(encoding='utf-8')).get('values', []), path.name, set())
        write_json(FLUID_HOST_TAG_TARGET / path.relative_to(source), {'values': values})
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


def asset_source(relative):
    """An original client asset: the hand-written ones are in src/main, the data-generated ones (item models of plain
    items, block states of generated blocks) in src/generated. A name is in at most one place; src/main wins."""
    for base in (ASSET_SOURCE, GENERATED_ASSETS):
        if (base / relative).exists():
            return base / relative
    return ASSET_SOURCE / relative


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
        source = asset_source(f'textures/{path}.png')
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
        for key, value in read(asset_source(f'models/{path}.json')).items():
            # render_type: 26.x derives the render layer from the texture's alpha (its own slime block model has none).
            if key in ('loader', 'retextured', '__comment', 'render_type'):
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
        model = read(asset_source(f'models/{path}.json'))
        return model.get('loader') not in (None, *PLAIN_LOADERS) or ('parent' in model and needs_loader(model['parent']))

    shutil.copyfile(ROOT / 'tconstruct/LICENSE', ASSET_TARGET / 'license')
    for name in sorted(path.stem for path in (HORIZON_DECLARATIONS / 'block').glob('*.json')):
        variants = read(asset_source(f'blockstates/{name}.json')).get('variants', {})
        if sorted(variants) == ['facing=east', 'facing=north', 'facing=south', 'facing=west']:
            variant = dict(variants['facing=north'])
        elif list(variants) == ['']:  # a block with no state of its own: its one model
            variant = dict(variants[''])
        else:
            raise SystemExit(f'block {name}: unexpected blockstate shape')
        if needs_loader(variant['model']):
            skipped.append(name)
            continue
        variant['model'] = import_model(variant['model'])
        write(f'blockstates/{name}.json', {'variants': {'': variant}})
        item_model = asset_source(f'models/item/{name}.json').exists()
        write(f'items/{name}.json', {'model': {'type': 'minecraft:model', 'model': import_model(f'tconstruct:item/{name}') if item_model else variant['model']}})
    for name in sorted(path.stem for path in (HORIZON_DECLARATIONS / 'item').glob('*.json')):
        if not asset_source(f'models/item/{name}.json').exists() or needs_loader(f'tconstruct:item/{name}'):
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


# The port declares the slime chestplate as `slime_chestplate`; the original registers (and models) it as `slimy_chestplate`.
MODEL_ALIASES = {'slime_chestplate': 'slimy_chestplate'}
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
    """Item definitions for the registered tools with material layers (armor, shields and wings included): the
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
        original = MODEL_SOURCE / 'item' / f'{MODEL_ALIASES.get(name, name)}.json'
        if not original.exists() or original_model(original).get('loader') != 'tconstruct:tool':
            left_out.append(name)
            continue
        modules = original_model(path)['modules']
        # Material slots: the parts of part_stats, or the stat types of material_stats (tools made of materials alone).
        slots = sum(len(m.get('parts') or m.get('stat_types') or []) for m in modules if m['type'] in ('tconstruct:part_stats', 'tconstruct:material_stats'))
        specs[name] = (original_model(original), slots)

    # The flag table: every modifier any tool state draws a sprite for, in id order.
    table = sorted({modifier for name, (data, _) in specs.items()
                    for ref in [f'tconstruct:item/{MODEL_ALIASES.get(name, name)}'] + [override['model'] for override in data.get('overrides', [])]
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
        root = f'tconstruct:item/{MODEL_ALIASES.get(name, name)}'
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


WORN_ROOTS = [GENERATED_ASSETS / 'textures/tinker_armor', ASSET_SOURCE / 'textures/tinker_armor']
WORN_TARGET = ASSET_TARGET / 'textures/entity/equipment'
ARMOR_TABLE = ROOT / 'tconstruct/mixins/tconstruct/api/src/armor_textures.rs'
EQUIPMENT_TARGET = ASSET_TARGET / 'equipment'
# Texture word of each equipment layer set (the original's `armor`, `leggings` and `wings` bases) and its texture folder.
WORN_KINDS = {'armor': 'humanoid', 'leggings': 'humanoid_leggings', 'wings': 'wings'}
# The material layers of each armor model in ArmorModelProvider (the dyed suppliers need the dyed modifier and are left out; the trim
# supplier is the game's own trim layer): ('fixed', prefix) is one base texture, ('material', prefix, slot) the texture of the material
# in that tool slot. Each prefix is completed with the layer set's word.
WORN_FAMILIES = {
    'travelers': [('fixed', 'travelers/base_'), ('material', 'travelers/cuirass_', 1), ('material', 'travelers/metal_', 0)],
    'plate': [('material', 'plate/plating_', 0), ('material', 'plate/maille_', 1)],
    'slime': [('material', 'slime/', 1), ('material', 'slime/overlay_', 0)],
    'slime_wings': [('material', 'slime/', 0)],
}
# Item path -> (family, layer-set word).
WORN_ITEMS = {
    **{f'travelers_{piece}': ('travelers', 'leggings' if piece == 'leggings' else 'armor') for piece in ('helmet', 'chestplate', 'leggings', 'boots')},
    **{f'plate_{piece}': ('plate', 'leggings' if piece == 'leggings' else 'armor') for piece in ('helmet', 'chestplate', 'leggings', 'boots')},
    **{f'slime_{piece}': ('slime', 'leggings' if piece == 'leggings' else 'armor') for piece in ('chestplate', 'leggings', 'boots')},
    'slime_wings': ('slime_wings', 'wings'),
}


def worn_source(stem):
    for root in WORN_ROOTS:
        if (root / f'{stem}.png').is_file():
            return root / f'{stem}.png'
    return None


def worn_word(stem):
    """The layer-set word a texture stem belongs to: the first of `armor`, `leggings`, `wings` in its file name."""
    name = stem.rsplit('/', 1)[1]
    return next(word for word in WORN_KINDS if re.search(rf'(^|_){word}(_|$)', name))


def worn_color(material, info):
    """The render info's vertex colour (0xRRGGBB) of the first redirect-chain material that has one."""
    chain, name = [], material
    while name is not None and name not in chain:
        chain.append(name)
        data = info.get(name, {})
        if 'color' in data:
            return int(data['color'], 16) & 0xFFFFFF
        target = data.get('texture')
        name = target.split(':', 1)[1] if target and target.startswith('tconstruct:') else None
    return None


def copy_armor_worn_assets():
    """Worn armor textures, equipment assets and the layer table of the armor renderer (armor_textures.rs).

    Mirrors the original's MaterialArmorTextureSupplier: a material's texture is the pre-baked `<base>_tconstruct_<material>` of its
    redirect chain, else the first fallback's `<base>_<fallback>` tinted with the material's vertex colour, else the base tinted the
    same way. Only the bases of the layer plans of the registered armor are read; files are copied byte for byte (SlimeKnights, MIT).
    Returns (textures copied, table rows).
    """
    sprites = material_sprites()
    info = {}
    for path in MATERIAL_INFO.rglob('*.json'):
        info['_'.join(path.relative_to(MATERIAL_INFO).with_suffix('').parts)] = json.loads(path.read_text(encoding='utf-8'))
    if 'unknown' in info:
        sprites['unknown'] = (['unknown'], list(info['unknown'].get('fallbacks', [])))
    words = {family: sorted({word for item_family, word in WORN_ITEMS.values() if item_family == family}) for family in WORN_FAMILIES}
    bases, rows, used = set(), [], set()
    for family, plan in WORN_FAMILIES.items():
        for word in words[family]:
            for layer in plan:
                base = f'{layer[1]}{word}'
                if worn_source(base) is None:
                    continue
                bases.add(base)
                used.add(base)
                if layer[0] == 'fixed':
                    continue
                for material, (chain, fallbacks) in sprites.items():
                    pick = next((f'_tconstruct_{name}' for name in chain if worn_source(f'{base}_tconstruct_{name}')), None)
                    tint = None
                    if pick is None:
                        pick = next((f'_{name}' for name in fallbacks if worn_source(f'{base}_{name}')), '')
                        tint = worn_color(material, info)
                    used.add(base + pick)
                    if pick or tint is not None:
                        rows.append((base, material, pick, tint))
    shutil.rmtree(WORN_TARGET, ignore_errors=True)
    for stem in sorted(used):
        destination = WORN_TARGET / WORN_KINDS[worn_word(stem)] / f'{stem}.png'
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(worn_source(stem), destination)
    # Equipment assets: the item's `equippable.asset_id`; each layer set names the plan's first base that exists (what draws when
    # the renderer declines), so every asset resolves to real textures.
    shutil.rmtree(EQUIPMENT_TARGET, ignore_errors=True)
    for family, plan in WORN_FAMILIES.items():
        layers = {}
        for word in words[family]:
            kind = WORN_KINDS[word]
            first = next((f'{layer[1]}{word}' for layer in plan if f'{layer[1]}{word}' in bases), None)
            if first:
                layers[kind] = [{'texture': f'tconstruct:{first}'}]
        write_json(EQUIPMENT_TARGET / f'{family}.json', {'layers': layers})
    rows.sort(key=lambda row: (row[0], row[1]))
    lines = ["//! The worn layers of the registered armor: which texture each material gives each layer, from the original's material render",
             '//! info and the pre-baked `tinker_armor` textures (SlimeKnights, MIT; see LICENSE). Generated by tools/import_original_data.py;',
             '//! `armor_worn.rs` reads it. A base absent from BASES is a layer the original skips for that layer set.', '',
             '/// One layer of an armor model: a base texture, or the texture of the material in a tool slot.',
             'pub(crate) enum Layer {', "    Fixed(&'static str),", "    Material(&'static str, usize),", '}', '',
             '/// Item path -> (family, layer-set word `armor`, `leggings` or `wings`).',
             'pub(crate) const ITEMS: &[(&str, &str, &str)] = &[']
    lines += [f'    ("{item}", "{family}", "{word}"),' for item, (family, word) in sorted(WORN_ITEMS.items())]
    lines += ['];', '', '/// The layers of each family, in draw order; each prefix is completed with the layer-set word.',
              'pub(crate) const FAMILIES: &[(&str, &[Layer])] = &[']
    for family, plan in WORN_FAMILIES.items():
        entries = [f'Layer::Fixed("{layer[1]}")' if layer[0] == 'fixed' else f'Layer::Material("{layer[1]}", {layer[2]})' for layer in plan]
        lines.append(f'    ("{family}", &[{", ".join(entries)}]),')
    lines += ['];', '', "/// Every base texture the original ships (a path under the layer set's texture folder, without `.png`), sorted.",
              'pub(crate) const BASES: &[&str] = &[']
    lines += [f'    "{base}",' for base in sorted(bases)]
    lines += ['];', '', '/// (base, material id path, suffix of the variant texture, tint 0xRRGGBB) of every material that does not simply use its',
              '/// untinted base; sorted by base and material.', 'pub(crate) const VARIANTS: &[(&str, &str, &str, Option<u32>)] = &[']
    lines += [f'    ("{base}", "{material}", "{suffix}", {"None" if tint is None else f"Some(0x{tint:06X})"}),' for base, material, suffix, tint in rows]
    lines += ['];']
    ARMOR_TABLE.write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
    return len(used), len(rows)


# --- World content: cobalt (unit U1) and the plain slime blocks (unit U2) of R4-world --------------------------------
# Source: world/TinkerWorld.java (registrations), shared/block/SlimeType.java, world/block/DirtType.java and the
# data-generated loot tables, features, biome modifiers and tags of the original.
GENERATED_DATA = ROOT / '.local/tconstruct-source/src/generated/resources/data'
CONTENT_DATA = ROOT / 'tconstruct/content/data'
# MapColor ids (0 to 61) as the host reads them; each one's colour was checked against the host's MAP_COLORS table.
MAP_COLOR = {'GRASS': 1, 'COLOR_ORANGE': 15, 'COLOR_PURPLE': 24, 'COLOR_BLUE': 25, 'DIAMOND': 31, 'NETHER': 35,
             'TERRACOTTA_ORANGE': 37, 'TERRACOTTA_LIGHT_BLUE': 39, 'WARPED_STEM': 56}
# SlimeType: map colour and light level. DirtType.TINKER: map colour.
SLIME_TYPES = {'earth': ('GRASS', 0), 'sky': ('DIAMOND', 0), 'ichor': ('COLOR_ORANGE', 10), 'ender': ('COLOR_PURPLE', 0)}
DIRT_TYPES = {'earth': 'GRASS', 'sky': 'WARPED_STEM', 'ichor': 'TERRACOTTA_LIGHT_BLUE', 'ender': 'TERRACOTTA_ORANGE'}
# Plain items: TinkerWorld's with Item.Properties (TooltipItem only adds the lang tooltip text) and TinkerCommons' tinker slime
# balls (`new Item(ITEM_PROPS)` for SlimeType.TINKER), which the congealed slime blocks drop.
WORLD_ITEMS = ['raw_cobalt', 'cobalt_shard', 'steel_shard', 'knightmetal_shard', 'sky_slime_ball', 'ichor_slime_ball', 'ender_slime_ball']
WORLD_FEATURES = ['cobalt_ore_small', 'cobalt_ore_large']
WORLD_BIOME_MODIFIERS = ['cobalt_ore']


def world_blocks():
    """name -> declaration of the plain-property world blocks, each from its Java registration. The template is the
    vanilla plain `Block` (stone: bass drum; dirt: the default harp, as Properties.of()) or the slime block; sounds name
    the vanilla block of the SoundType. Light is declared only above zero (the templates emit none)."""
    def block(template, color, sounds, hardness, resistance, requires_tool, light=0, **extra):
        properties = {'hardness': hardness, 'explosion_resistance': resistance, 'sounds': sounds, 'map_color': MAP_COLOR[color], **extra}
        if template != 'minecraft:slime_block' or requires_tool:
            properties['requires_tool'] = requires_tool
        if light:
            properties['light'] = light
        return {'template': template, 'properties': properties, 'item': True}

    ore = 'minecraft:nether_gold_ore'  # SoundType.NETHER_ORE
    blocks = {
        # builder(MapColor.NETHER, NETHER_ORE).instrument(BASEDRUM).requiresCorrectToolForDrops().strength(10.0F)
        'cobalt_ore': block('minecraft:stone', 'NETHER', ore, 10.0, 10.0, True),
        # builder(MapColor.COLOR_BLUE, NETHER_ORE).instrument(BASEDRUM).requiresCorrectToolForDrops().strength(6.0f, 7.0f)
        'raw_cobalt_block': block('minecraft:stone', 'COLOR_BLUE', ore, 6.0, 7.0, True),
    }
    for slime, (color, light) in SLIME_TYPES.items():
        # congealed: builder(color, SLIME_BLOCK).strength(0.5F).friction(0.5F).lightLevel(type light)
        blocks[f'{slime}_congealed_slime'] = block('minecraft:dirt', color, 'minecraft:slime_block', 0.5, 0.5, False, light, friction=0.5)
        if slime != 'earth':  # the earth slime block is vanilla's
            # slimeProps: builder(color, SLIME_BLOCK).friction(0.8F).noOcclusion() (no strength: 0); ichor adds its light
            blocks[f'{slime}_slime'] = block('minecraft:slime_block', color, 'minecraft:slime_block', 0.0, 0.0, False, light, friction=0.8, occludes=False)
    for dirt, color in DIRT_TYPES.items():
        # builder(color, SLIME_BLOCK).strength(1.9f); harvest tiers are tags and need no tool (no requiresCorrectToolForDrops)
        blocks[f'{dirt}_slime_dirt'] = block('minecraft:dirt', color, 'minecraft:slime_block', 1.9, 1.9, False)
    return blocks


def write_world_declarations():
    """Block and item declarations of the world units, written beside the hand-written station ones."""
    blocks = world_blocks()
    for name, declaration in blocks.items():
        write_json(HORIZON_DECLARATIONS / 'block' / f'{name}.json', declaration, compact=True)
    for name in WORLD_ITEMS:
        write_json(HORIZON_DECLARATIONS / 'item' / f'{name}.json', {'template': 'minecraft:paper'}, compact=True)
    return len(blocks), len(WORLD_ITEMS)


def whole(value):
    """An integral float as the integer 26.3 writes (rolls 1.0 -> 1), recursively."""
    if isinstance(value, float) and value == int(value):
        return int(value)
    if isinstance(value, list):
        return [whole(item) for item in value]
    if isinstance(value, dict):
        return {key: whole(item) for key, item in value.items()}
    return value


def upgrade_loot(node):
    """A 1.20.1 loot table in 26.3's shape (as horizon-port-pack's carry does): `conditions` lists become one `condition`
    (`all_of` for several), `functions` become `modifier`, kinds are named by `type`; a match_tool on silk touch alone
    is `minecraft:tool/can_silk_touch`. Only the shapes the world blocks use are known: anything else stops the import."""
    if isinstance(node, list):
        return [upgrade_loot(item) for item in node]
    if not isinstance(node, dict):
        return node
    result = {}
    for key, value in node.items():
        if key == 'conditions':
            terms = [upgrade_condition(term) for term in value]
            result['condition'] = terms[0] if len(terms) == 1 else {'type': 'minecraft:all_of', 'terms': terms}
        elif key == 'functions':
            result['modifier'] = [upgrade_function(function) for function in value]
        elif key == 'bonus_rolls' and value == 0:
            continue  # the default
        elif key in ('type', 'name', 'random_sequence', 'rolls'):
            result[key] = value
        elif key in ('pools', 'entries', 'children'):
            result[key] = upgrade_loot(value)
        else:
            raise SystemExit(f'loot table: unknown key {key!r}')
    return result


def upgrade_condition(condition):
    kind = condition['condition']
    if kind == 'minecraft:survives_explosion' and set(condition) == {'condition'}:
        return {'type': kind}
    silk = {'enchantments': [{'enchantment': 'minecraft:silk_touch', 'levels': {'min': 1}}]}
    if kind == 'minecraft:match_tool' and condition['predicate'] == silk:
        return 'minecraft:tool/can_silk_touch'
    raise SystemExit(f'loot table: unknown condition {condition}')


def upgrade_function(function):
    kind = function['function']
    rest = {key: value for key, value in function.items() if key != 'function'}
    if kind == 'minecraft:explosion_decay' and not rest:
        return {'type': kind}
    if kind == 'minecraft:apply_bonus' and rest == {'enchantment': 'minecraft:fortune', 'formula': 'minecraft:ore_drops'}:
        return {'type': kind, **rest}
    if kind == 'minecraft:set_count' and rest.get('add') is False and set(rest) == {'add', 'count'}:
        return {'type': kind, 'count': rest['count']}
    raise SystemExit(f'loot table: unknown function {function}')


def copy_world_loot(names):
    """Block loot tables (data/<ns>/loot_table/blocks, 26.3's folder) of the given blocks."""
    target = CONTENT_DATA / 'tconstruct/loot_table'
    shutil.rmtree(target, ignore_errors=True)
    for name in names:
        table = whole(upgrade_loot(json.loads((GENERATED_DATA / f'tconstruct/loot_tables/blocks/{name}.json').read_text(encoding='utf-8'))))
        write_json(target / 'blocks' / f'{name}.json', table)
    return len(names)


def upgrade_feature(feature):
    """A 1.20.1 configured feature in 26.3's `worldgen/feature` form: the config is the feature's own body and a plain
    block state is its name."""
    body = {'type': feature['type'], **feature['config']}
    if body['type'] != 'minecraft:ore':
        raise SystemExit(f'world feature: only minecraft:ore is imported, not {body["type"]}')
    for target in body['targets']:
        state = target['state']
        target['state'] = state['Name'] if list(state) == ['Name'] else state
    return body


def copy_world_features():
    """Cobalt's configured and placed features and its biome modifier (the host reads `neoforge/biome_modifier` and the
    `forge:`/`neoforge:` types alike; the original's `forge:add_features` becomes `neoforge:add_features`)."""
    for folder in ('worldgen', 'neoforge'):
        shutil.rmtree(CONTENT_DATA / 'tconstruct' / folder, ignore_errors=True)
    source = GENERATED_DATA / 'tconstruct'
    for name in WORLD_FEATURES:
        feature = upgrade_feature(json.loads((source / f'worldgen/configured_feature/{name}.json').read_text(encoding='utf-8')))
        for target in feature['targets']:
            block = target['state'] if isinstance(target['state'], str) else target['state']['Name']
            if block.startswith('tconstruct:') and not (HORIZON_DECLARATIONS / 'block' / f'{block[len("tconstruct:"):]}.json').exists():
                raise SystemExit(f'feature {name} places the undeclared block {block}')
        write_json(CONTENT_DATA / f'tconstruct/worldgen/feature/{name}.json', feature)
        placed = json.loads((source / f'worldgen/placed_feature/{name}.json').read_text(encoding='utf-8'))
        if placed['feature'] != f'tconstruct:{name}':
            raise SystemExit(f'placed feature {name} names {placed["feature"]}')
        write_json(CONTENT_DATA / f'tconstruct/worldgen/placed_feature/{name}.json', placed)
    for name in WORLD_BIOME_MODIFIERS:
        modifier = json.loads((source / f'forge/biome_modifier/{name}.json').read_text(encoding='utf-8'))
        if not modifier['type'].startswith('forge:') or not set(modifier['features']) <= {f'tconstruct:{n}' for n in WORLD_FEATURES}:
            raise SystemExit(f'biome modifier {name} is not a forge modifier of the shipped features')
        modifier['type'] = 'neoforge:' + modifier['type'].removeprefix('forge:')
        write_json(CONTENT_DATA / f'tconstruct/neoforge/biome_modifier/{name}.json', modifier)
    return len(WORLD_FEATURES) * 2 + len(WORLD_BIOME_MODIFIERS)


def copy_vanilla_tags():
    """The original's additions to vanilla's block tags (mineable/*, needs_*_tool, enderman_holdable...), reduced to the
    declared blocks and to the Tinkers block tags that ship (the host drops unknown ids anyway; this keeps the files to what this
    port registers, and a later unit's blocks join on the next import). Entries of any other namespace stop the import,
    Forge's convention tags are left out (that namespace does not exist here, as in copy_tags). `forge:needs_gold_tool` has no
    26.3 tag: gold tools are wooden ones there, and no world block needs the tool to drop anyway. Vanilla's item tags (tool
    families, logs...) wait for the units that register those items."""
    target = CONTENT_DATA / 'minecraft/tags/block'
    declared = {p.stem for p in (HORIZON_DECLARATIONS / 'block').glob('*.json')}
    shipped = {path.relative_to(TAG_TARGET / 'block').with_suffix('').as_posix() for path in (TAG_TARGET / 'block').rglob('*.json')}
    shutil.rmtree(target, ignore_errors=True)
    folder = GENERATED_DATA / 'minecraft/tags/blocks'
    count = 0
    for path in sorted(folder.rglob('*.json')):
        values = []
        for entry in json.loads(path.read_text(encoding='utf-8')).get('values', []):
            value = entry if isinstance(entry, str) else entry['id']  # an optional entry is {id, required}
            if value.startswith('#tconstruct:'):
                keep = value[len('#tconstruct:'):] in shipped
            elif value.startswith('tconstruct:'):
                keep = value[len('tconstruct:'):] in declared
            elif value.lstrip('#').startswith('forge:'):
                keep = False
            else:
                raise SystemExit(f'{path.name}: entry {value} is neither a Tinkers nor a Forge one')
            if keep:
                values.append(entry)
        if values:
            write_json(target / path.relative_to(folder), {'values': values})
            count += 1
    return count


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
    fluid_count, fluid_names = write_fluid_defs()
    tags = copy_tags() + copy_damage_tags() + copy_fluid_host_tags(fluid_names)
    write_damage_vanilla()
    languages = copy_lang()
    world_blocks_written, world_items_written = write_world_declarations()
    models, textures, unrendered = copy_client_assets()
    world_loot = copy_world_loot(sorted(world_blocks()))
    world_worldgen = copy_world_features()
    world_tags = copy_vanilla_tags()
    part_names, part_models, part_textures = copy_part_assets()
    tool_names, tool_models, tool_textures, tools_left_out = copy_tool_assets()
    worn_textures, worn_rows = copy_armor_worn_assets()
    models, textures = len(list((ASSET_TARGET / 'models').rglob('*.json'))), len(list((ASSET_TARGET / 'textures').rglob('*.png')))
    unrendered = [name for name in unrendered if name not in part_names + tool_names]
    recipes = copy_material_recipes(skipped_materials) + copy_modifier_recipes() + copy_part_builder_recipes() + copy_recycling_recipes() + copy_severing_recipes() + copy_fluid_recipes() + copy_fluid_tags()
    print(f'copied {copied} files, {tags} tags, {languages} language files and {recipes} material recipes from {head}; {models} models and {textures} textures (left out, need the original tool loader: {len(unrendered)}; tools with no layered item definition: {tools_left_out}); {worn_textures} worn armor textures and {worn_rows} material variants; {len(types)} modifier module types; excluded materials: {sorted(skipped_materials)}')


if __name__ == '__main__':
    sys.exit(main())
