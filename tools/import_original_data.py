#!/usr/bin/env python3
"""Copy the original mod's generated tinkering data into the clean content root.

Source: the read-only checkout at .local/tconstruct-source (revision pinned by
tconstruct/compatibility.json). Files are copied byte for byte, never edited, so the content
root stays a mechanical projection of the original. Other-mod integration is left out
only where the original registers content on another mod's behalf: the minotaur axe.
The sixteen integration materials are copied with everything else, condition payload
included; the port evaluates data conditions at load, so they stay inactive until
another mod fills their ingot tags (or force_integration_materials is set), as in Java.
One divergence from Java: this port bakes its creative tabs from the shipped data
rather than the loaded registry, so the sixteen appear in the tab tables while inactive.
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
# No tool definition is excluded: the minotaur axe ships unconditionally (the original's
# Twilight Forest condition cannot be expressed in the port's data).
EXCLUDED_TOOLS = set()
# The original's Forge-era tag folders, renamed to the singular names the pinned host reads.
TAG_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/tags'
TAG_TARGET = ROOT / 'tconstruct/content/data/tconstruct/tags'
TAG_FOLDER_NAMES = {'items': 'item', 'blocks': 'block', 'entity_types': 'entity_type',
                    'block_entity_type': 'block_entity_type', 'potion': 'potion', 'menu': 'menu'}


REPAIRABLE = ['head', 'limb', 'plating_helmet', 'plating_chestplate', 'plating_leggings', 'plating_boots', 'plating_shield', 'skull', 'slime']


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
    # FakeIngotItem is a RepairKitItem for the compatibility materials that lack an ingot form (its row is
    # intersected with that material tag below, and with whether the material is shipped at all).
    parts.append(('fake_ingot', 'repair_kit'))
    compat = set()
    compat_path = TARGET / 'tags/materials/compatibility_metals.json'
    if compat_path.exists():
        for entry in json.loads(compat_path.read_text(encoding='utf-8'))['values']:
            text = entry if isinstance(entry, str) else entry.get('id', '')
            if text.startswith('tconstruct:') and '/' not in text:
                compat.add(text.split(':', 1)[1])
    lines = ['//! Tool parts with the visible shipped materials that can make them, in creative-tab order.',
             '//! Generated from the original data by tools/import_original_data.py; do not edit by hand.', '',
             'pub(crate) const PARTS: &[(&str, &[&str])] = &[']
    for item, stat in parts:
        # RepairKitItem.canUseMaterial: the repair kit stat or any stat type of IRepairableMaterialStats
        wanted = {f'tconstruct:{stat}'} | ({f'tconstruct:{name}' for name in REPAIRABLE} if stat == 'repair_kit' else set())
        materials = [name for name in order if wanted & stats.get(name, set())]
        if item == 'fake_ingot':
            # FakeIngotItem.canUseMaterial: only the compatibility materials (the missing-item half of the
            # check needs the loaded material recipes, so a material with an ingot recipe may still be listed).
            materials = [name for name in materials if name in compat]
        lines.append(f'    ("{item}", &[' + ', '.join(f'"{name}"' for name in materials) + ']),')
    lines.append('];')
    # ModifierCrystalItem.addVariants: every modifier a modifier recipe makes (ModifierRecipeLookup.getRecipeModifierList, sorted by id),
    # except the extract blacklist; the original reads them from the loaded recipes, the tab here is fixed when the mod starts.
    def tag_members(path, seen=()):
        found = set()
        for entry in json.loads(path.read_text(encoding='utf-8')).get('values', []):
            text = entry if isinstance(entry, str) else entry['id']
            if text.startswith('#tconstruct:') and (TARGET / 'tags/modifiers' / f'{text[12:]}.json').is_file():
                found |= tag_members(TARGET / 'tags/modifiers' / f'{text[12:]}.json')
            elif text.startswith('tconstruct:'):
                found.add(text)
        return found
    blacklist = tag_members(TARGET / 'tags/modifiers/extract_blacklist/tools.json')
    modifier_made = set()
    for path in MODIFIER_RECIPE_SOURCE.rglob('*.json'):
        data = json.loads(path.read_text(encoding='utf-8'))
        if data.get('type') in ('tconstruct:modifier', 'tconstruct:incremental_modifier', 'tconstruct:multilevel_modifier', 'tconstruct:multilevel_incremental_modifier', 'tconstruct:swappable_modifier'):
            result = data['result']
            modifier_made.add(result if isinstance(result, str) else result['name'])
    crystals = sorted(name for name in modifier_made if name not in blacklist)
    lines += ['', '/// Modifiers that get a modifier crystal stack, by id (namespace tconstruct).', 'pub(crate) const CRYSTALS: &[&str] = &[']
    lines += [f'    "{name}",' for name in crystals] + ['];']
    (ROOT / 'tconstruct/mixins/tconstruct/api/src/creative_parts.rs').write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')


# TinkerTools.addTabItems order: tools that lack materials, small tools, broad tools, ranged, ancient tools, then armor.
# The minotaur axe closes the ancient tools, as in TinkerTools.addTabItems: the original registers the item itself only
# when Twilight Forest is loaded, but this port registers it, so it is listed like every other tool.
TOOL_ORDER = ['flint_and_brick', 'sky_staff', 'earth_staff', 'ichor_staff', 'ender_staff',
              'pickaxe', 'pickadze', 'mattock', 'hand_axe', 'kama', 'dagger', 'sword',
              'sledge_hammer', 'vein_hammer', 'excavator', 'broad_axe', 'scythe', 'cleaver',
              'crossbow', 'longbow', 'fishing_rod', 'javelin', 'arrow', 'shuriken', 'throwing_axe',
              'melting_pan', 'war_pick', 'battlesign', 'swasher', 'minotaur_axe',
              'travelers_helmet', 'travelers_chestplate', 'travelers_leggings', 'travelers_boots', 'travelers_shield',
              'plate_helmet', 'plate_chestplate', 'plate_leggings', 'plate_boots', 'plate_shield',
              'slime_helmet', 'slime_chestplate', 'slime_leggings', 'slime_boots', 'slime_wings']


# The tab names items, while the definitions are opened by definition name: the one tool whose
# item differs from its definition (see item_for_definition in tool_definitions.rs).
TOOL_ITEM_NAMES = {'slime_chestplate': 'slimy_chestplate'}


def write_creative_fluids(fluid_names):
    """creative_fluids.rs: the fluids of the Tanks tab in registry order, each with whether the original (TankItem.addFilledVariants)
    fills the ingot tank and gauge for it (the `tooltips/metal` fluid tag) instead of the fuel ones; `hide_in_creative_tanks` ones are left out."""
    def members(name):
        data = json.loads((FLUID_HOST_TAG_TARGET / f'{name}.json').read_text(encoding='utf-8'))
        found = set()
        for entry in data['values']:
            text = entry if isinstance(entry, str) else entry['id']
            if text.startswith('#tconstruct:') and (FLUID_HOST_TAG_TARGET / f'{text[12:]}.json').is_file():
                found |= members(text[12:])
            elif not text.startswith('#'):
                found.add(text.removeprefix('tconstruct:'))
        return found
    metal, hidden = members('tooltips/metal'), members('hide_in_creative_tanks')
    lines = ['//! The fluids of the Tanks creative tab, in registry order, and whether each is a metal (ingot tanks instead of fuel tanks).',
             '//! Generated from the original data by tools/import_original_data.py; do not edit by hand.', '',
             'pub(crate) const FLUIDS: &[(&str, bool)] = &[']
    for name in fluid_names:
        if name not in hidden:
            lines.append('    ("%s", %s),' % (name, 'true' if name in metal else 'false'))
    lines.append('];')
    (ROOT / 'tconstruct/mixins/tconstruct/api/src/creative_fluids.rs').write_text(chr(10).join(lines) + chr(10), encoding='utf-8', newline=chr(10))


def write_creative_tools():
    """creative_tools.rs: the stacks of the Tools tab, as ToolBuildHandler.addVariants lists them (a tool with no materials is one plain stack).

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
    lines = ['//! Stacks of the Tools creative tab: per tool, the material of each slot for every entry.',
             '//! Generated from the original data by tools/import_original_data.py; do not edit by hand.', '',
             'pub(crate) const TOOLS: &[(&str, &[&[&str]])] = &[']
    for tool in TOOL_ORDER:
        modules = json.loads((TARGET / 'tool_definitions' / f'{tool}.json').read_text(encoding='utf-8'))['modules']
        required = []
        for module in modules:
            if module['type'] == 'tconstruct:part_stats':
                required = [part_stat[(part if isinstance(part, str) else part['item']).split(':')[1]] for part in module['parts']]
            elif module['type'] == 'tconstruct:material_stats':
                required = [(entry if isinstance(entry, str) else entry['stat']).split(':')[1] for entry in module['stat_types']]
        has = lambda name, stat: f'tconstruct:{stat}' in stats.get(name, set())
        first = [next(name for name in order if has(name, stat)) for stat in required]
        entries = []
        for name in order:
            if any(has(name, stat) for stat in required):
                entries.append([name if has(name, stat) else first[i] for i, stat in enumerate(required)])
        if not required:
            entries = [[]]  # no materials: one plain stack
        rows = ', '.join('&[' + ', '.join(f'"{m}"' for m in entry) + ']' for entry in entries)
        lines.append(f'    ("{TOOL_ITEM_NAMES.get(tool, tool)}", &[{rows}]),')
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
        if json.loads(path.read_text(encoding='utf-8')).get('type') not in ('tconstruct:modifier', 'tconstruct:incremental_modifier', 'tconstruct:swappable_modifier', 'tconstruct:modifier_salvage', 'tconstruct:remove_modifier', 'tconstruct:multilevel_modifier', 'tconstruct:multilevel_incremental_modifier', 'tconstruct:overslime_modifier', 'tconstruct:enchantment_converting', 'tconstruct:modifier_repair', 'tconstruct:armor_dyeing_modifier', 'tconstruct:extract_modifier', 'tconstruct:modifier_set_worktable', 'tconstruct:modifier_sorting', 'tconstruct:toggle_interaction'):
            continue
        destination = MODIFIER_RECIPE_TARGET / path.relative_to(MODIFIER_RECIPE_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    return count


DAMAGING_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes/tables/tinker_station_damaging'
DAMAGING_TARGET = ROOT / 'tconstruct/content/data/tconstruct/recipes/tables/tinker_station_damaging'


def copy_table_recipes():
    """The fieldless station repair gate and the part swapping recipes live in the tables
    folder beside the material swapping data, so they are copied file by file."""
    tables = {"tinker_station_repair", "tinker_station_part_swapping", "ammo_part_swapping", "throwing_axe_part_swapping"}
    types = {"tconstruct:tinker_station_repair", "tconstruct:tinker_station_part_swapping"}
    written = 0
    for name in tables:
        source = GENERATED / "recipes/tables" / f"{name}.json"
        payload = json.loads(source.read_text(encoding="utf-8"))
        if payload.get("type") not in types:
            continue
        target = CONTENT / "recipes/tables" / f"{name}.json"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(source.read_bytes())
        written += 1
    return written


def copy_building_recipes():
    """Tool building and tipped transform recipes ship from the original building folder."""
    source_root = GENERATED / "recipes/tools/building"
    target_root = CONTENT / "recipes/tools/building"
    if target_root.exists():
        shutil.rmtree(target_root)
    written = 0
    for source in sorted(source_root.rglob("*.json")):
        payload = json.loads(source.read_text(encoding="utf-8"))
        if payload.get("type") not in {"tconstruct:tool_building", "tconstruct:tipped_tool_transform"}:
            continue
        target = target_root / source.relative_to(source_root)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(source.read_bytes())
        written += 1
    return written



def copy_damaging_recipes():
    """The tinker station damaging recipes (`tconstruct:tinker_station_damaging`: tools damaged with
    lava, blazing blood, magma or venom), byte for byte."""
    if DAMAGING_TARGET.exists():
        shutil.rmtree(DAMAGING_TARGET)
    count = 0
    for path in sorted(DAMAGING_SOURCE.rglob('*.json')):
        if json.loads(path.read_text(encoding='utf-8')).get('type') != 'tconstruct:tinker_station_damaging':
            continue
        destination = DAMAGING_TARGET / path.relative_to(DAMAGING_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    return count


def copy_enchantment_data():
    """The enchantment-to-modifier map and the modifier_like enchantment tags its # keys name, byte
    for byte. Every tag entry names another mod's enchantment as optional, so the tags resolve empty
    in this game; they are kept so a datapack defining one of those enchantments makes the mapping live."""
    shutil.copyfile(SOURCE / 'enchantments_to_modifiers.json', TARGET / 'enchantments_to_modifiers.json')
    source = TAG_SOURCE / 'enchantment/modifier_like'
    target = TAG_TARGET / 'enchantment/modifier_like'
    if target.exists():
        shutil.rmtree(target)
    count = 0
    for path in sorted(source.glob('*.json')):
        target.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target / path.name)
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


DAMAGEABLE_RECYCLE_SOURCES = [
    (ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes/tools/recycling',
     ROOT / 'tconstruct/content/data/tconstruct/recipes/tools/recycling'),
    (ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes/tables/recycling',
     ROOT / 'tconstruct/content/data/tconstruct/recipes/tables/recycling'),
]


def copy_damageable_recycling_recipes():
    """The part builder's damageable recycling recipes (`tconstruct:part_builder_recycling`: a vanilla item or a
    material-less tool breaks into fixed results, scaled by its remaining durability), byte for byte. They sit
    beside the tool recycling recipes in the tools tree and alone in the tables tree."""
    count = 0
    for source, target in DAMAGEABLE_RECYCLE_SOURCES:
        for path in sorted(source.rglob('*.json')):
            if json.loads(path.read_text(encoding='utf-8')).get('type') != 'tconstruct:part_builder_recycling':
                continue
            destination = target / path.relative_to(source)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)
            count += 1
    return count


LAYOUT_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/tinkering/station_layouts'
LAYOUT_TARGET = ROOT / 'tconstruct/content/data/tconstruct/tinkering/station_layouts'
# The layouts TinkerTables.commonSetup registers as required: the main layout of each station block.
REQUIRED_LAYOUTS = ['tinker_station', 'tinkers_anvil', 'scorched_anvil']


def copy_station_layouts():
    """The tinker station slot layouts, byte for byte, plus station_layout_order.rs: the tab order of the
    non-main layouts (StationSlotLayoutLoader's sorted list: by sortIndex, then name), which the window
    publishes button indices into. A layout without a sortIndex is a station's main layout."""
    if LAYOUT_TARGET.exists():
        shutil.rmtree(LAYOUT_TARGET)
    LAYOUT_TARGET.mkdir(parents=True)
    layouts = {}
    for path in sorted(LAYOUT_SOURCE.glob('*.json')):
        shutil.copyfile(path, LAYOUT_TARGET / path.name)
        layouts[path.stem] = json.loads(path.read_text(encoding='utf-8'))
    missing = [name for name in REQUIRED_LAYOUTS if name not in layouts]
    if missing:
        raise SystemExit(f'required station layouts missing: {missing}')
    tabs = sorted((data['sortIndex'], name) for name, data in layouts.items() if 'sortIndex' in data)
    lines = ['//! The station layouts the window offers as tabs, in the loader\'s order (sortIndex, then name).',
             '//! Generated from the original data by tools/import_original_data.py; do not edit by hand.', '',
             'pub(crate) const TAB_ORDER: &[&str] = &[']
    lines += [f'    "{name}",' for _, name in tabs] + ['];']
    (ROOT / 'tconstruct/mixins/tconstruct/api/src/station_layout_order.rs').write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
    return len(layouts)


SEVERING_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes/tools/severing'
SEVERING_TARGET = ROOT / 'tconstruct/content/data/tconstruct/recipes/tools/severing'
LOOT_MODIFIER_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/loot_modifiers'
LOOT_MODIFIER_TARGET = ROOT / 'tconstruct/content/data/tconstruct/loot_modifiers'
LOOT_INJECTOR_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/mantle/loot_injectors'
LOOT_INJECTOR_TARGET = ROOT / 'tconstruct/content/data/tconstruct/mantle/loot_injectors'
GLOBAL_LOOT_MODIFIER_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/forge/loot_modifiers/global_loot_modifiers.json'
GLOBAL_LOOT_MODIFIER_TARGET = ROOT / 'tconstruct/content/data/forge/loot_modifiers/global_loot_modifiers.json'


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


def copy_loot_modifiers():
    """The global loot modifiers and the list that orders them, byte for byte. The list keeps its Forge-era
    folder: the port's own loot engine reads it to learn the order the modifiers run in."""
    if LOOT_MODIFIER_TARGET.exists():
        shutil.rmtree(LOOT_MODIFIER_TARGET)
    count = 0
    for path in sorted(LOOT_MODIFIER_SOURCE.rglob('*.json')):
        destination = LOOT_MODIFIER_TARGET / path.relative_to(LOOT_MODIFIER_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    GLOBAL_LOOT_MODIFIER_TARGET.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(GLOBAL_LOOT_MODIFIER_SOURCE, GLOBAL_LOOT_MODIFIER_TARGET)
    return count + 1


def copy_loot_injectors():
    """The Mantle loot injectors and nothing else: the entries the port merges into vanilla tables
    at load, byte for byte (the port's own loot engine evaluates their top-level conditions)."""
    if LOOT_INJECTOR_TARGET.exists():
        shutil.rmtree(LOOT_INJECTOR_TARGET)
    count = 0
    for path in sorted(LOOT_INJECTOR_SOURCE.rglob('*.json')):
        destination = LOOT_INJECTOR_TARGET / path.relative_to(LOOT_INJECTOR_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    return count


def copy_part_builder_recipes():
    """The part builder recipes, byte for byte: the material part recipes and the fixed-material
    item recipes (the plating dummies). The sand and red sand cast recipes belong to the smeltery
    and come with it. The tools/building folder's one item_part_builder recipe (elytra_copying:
    dragon scales onto an elytra) joins them here, the folder part_builder_recipes.rs reads; the
    folder's tool_building recipes have no reader, as tools are built from their definitions, so
    they stay out."""
    if PART_RECIPE_TARGET.exists():
        shutil.rmtree(PART_RECIPE_TARGET)
    count = 0
    for path in sorted(PART_RECIPE_SOURCE.rglob('*.json')):
        if json.loads(path.read_text(encoding='utf-8')).get('type') not in ('tconstruct:part_builder', 'tconstruct:item_part_builder'):
            continue
        destination = PART_RECIPE_TARGET / path.relative_to(PART_RECIPE_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    building = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes/tools/building'
    for path in sorted(building.rglob('*.json')):
        if json.loads(path.read_text(encoding='utf-8')).get('type') != 'tconstruct:item_part_builder':
            continue
        destination = PART_RECIPE_TARGET / path.name
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
    """The smeltery domain's recipes, byte for byte, under recipes/fluids with the original's relative path,
    the compat folder with the rest: its recipes carry their forge:mod_loaded load conditions, which the
    loader evaluates, as the integration alloys' mantle:tag_filled conditions already ship. A forge:conditional
    file is kept when any wrapped recipe is of a listed type. The sand and clay cast pattern recipes of the
    part builder (item_part_builder) in smeltery/casts come with them."""
    if FLUID_RECIPE_TARGET.exists():
        shutil.rmtree(FLUID_RECIPE_TARGET)
    count = 0
    for path in sorted(FLUID_RECIPE_SOURCE.rglob('*.json')):
        relative = path.relative_to(FLUID_RECIPE_SOURCE)
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
    # Mantle defines `mantle:water` itself (the recipes of composite oxidation and tipped arrow cleaning name it); its own definition is water.
    write_json(FLUID_TAG_TARGET / 'mantle/water.json', {'values': ['minecraft:water', 'minecraft:flowing_water']})
    return count + 1


FLUID_EFFECT_SOURCE = SOURCE / 'fluid_effects'
FLUID_EFFECT_TARGET = TARGET / 'fluid_effects'


def copy_fluid_effects():
    """The fluid effects (tinkering/fluid_effects), byte for byte: the engine in fluid_effects.rs decodes them with
    the original's FluidEffectManager semantics, load conditions and all."""
    if FLUID_EFFECT_TARGET.exists():
        shutil.rmtree(FLUID_EFFECT_TARGET)
    count = 0
    for path in sorted(FLUID_EFFECT_SOURCE.rglob('*.json')):
        destination = FLUID_EFFECT_TARGET / path.relative_to(FLUID_EFFECT_SOURCE)
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


# FluidDeferredRegisterExtension: level decrease per block, slope find distance and tick rate of each family (Forge's defaults: 1, 4, 5).
FLUID_FAMILIES = {'register': (1, 4, 5), 'registerSlime': (1, 4, 50), 'registerStone': (2, 2, 40), 'registerMetal': (3, 3, 30), 'registerGem': (4, 4, 45), 'registerGlass': (8, 8, 30)}
# net.minecraft.world.level.material.MapColor ids
MAP_COLORS = {name: index for index, name in enumerate(
    'NONE GRASS SAND WOOL FIRE ICE METAL PLANT SNOW CLAY DIRT STONE WATER WOOD QUARTZ COLOR_ORANGE COLOR_MAGENTA COLOR_LIGHT_BLUE COLOR_YELLOW COLOR_LIGHT_GREEN '
    'COLOR_PINK COLOR_GRAY COLOR_LIGHT_GRAY COLOR_CYAN COLOR_PURPLE COLOR_BLUE COLOR_BROWN COLOR_GREEN COLOR_RED COLOR_BLACK GOLD DIAMOND LAPIS EMERALD PODZOL NETHER '
    'TERRACOTTA_WHITE TERRACOTTA_ORANGE TERRACOTTA_MAGENTA TERRACOTTA_LIGHT_BLUE TERRACOTTA_YELLOW TERRACOTTA_LIGHT_GREEN TERRACOTTA_PINK TERRACOTTA_GRAY '
    'TERRACOTTA_LIGHT_GRAY TERRACOTTA_CYAN TERRACOTTA_PURPLE TERRACOTTA_BLUE TERRACOTTA_BROWN TERRACOTTA_GREEN TERRACOTTA_RED TERRACOTTA_BLACK CRIMSON_NYLIUM '
    'CRIMSON_STEM CRIMSON_HYPHAE WARPED_NYLIUM WARPED_STEM WARPED_HYPHAE WARPED_WART_BLOCK DEEPSLATE RAW_IRON GLOW_LICHEN'.split())}
FLUID_EFFECTS = {'MobEffects.POISON': 'minecraft:poison', 'MobEffects.MOVEMENT_SLOWDOWN': 'minecraft:slowness', 'MobEffects.BLINDNESS': 'minecraft:blindness',
                 'TinkerEffects.bouncy.get()': 'tconstruct:bouncy', 'TinkerEffects.ricochet.get()': 'tconstruct:ricochet', 'TinkerEffects.enderference.get()': 'tconstruct:enderference'}


def duration(text):
    """A MobEffectInstance duration such as `5*20` or `250`."""
    if not re.fullmatch(r'[0-9 *]+', text):
        raise SystemExit(f'effect duration {text}')
    result = 1
    for factor in text.split('*'):
        result *= int(factor)
    return result


def fluid_block(name, rest):
    """The block of one registration: None when it has none (`.unplacable()`), else its map colour, light and contact rule."""
    burning = re.search(r'\.burningBlock\(MapColor\.(\w+),\s*(\d+),\s*(\d+),\s*([\d.]+)f\)', rest)
    if burning:
        return {'color': burning.group(1), 'light': int(burning.group(2)), 'contact': ('burning', int(burning.group(3)), float(burning.group(4)))}
    start = rest.find('.block(')
    if start < 0:
        if '.unplacable()' in rest:
            return None
        raise SystemExit(f'fluid {name}: block not understood: {rest}')
    depth, index = 0, start + len('.block')
    for index in range(start + len('.block'), len(rest)):
        depth += rest[index] == '('
        depth -= rest[index] == ')'
        if depth == 0:
            break
    inner = rest[start + len('.block(') : index]
    plain = re.fullmatch(r'MapColor\.(\w+),\s*(\d+)', inner)
    if plain:
        return {'color': plain.group(1), 'light': int(plain.group(2)), 'contact': None}
    burn = re.fullmatch(r'createBurning\(MapColor\.(\w+),\s*(\d+),\s*(\d+),\s*([\d.]+)f\)', inner)
    if burn:
        return {'color': burn.group(1), 'light': int(burn.group(2)), 'contact': ('burning', int(burn.group(3)), float(burn.group(4)))}
    effect = re.fullmatch(r'createEffect\(MapColor\.(\w+),\s*(\d+),\s*\(\)\s*->\s*new MobEffectInstance\(([\w.()]+),\s*([0-9 *]+)(?:,\s*(\d+))?\)\)', inner)
    if not effect or effect.group(3) not in FLUID_EFFECTS:
        raise SystemExit(f'fluid {name}: block not understood: {inner}')
    block = {'color': effect.group(1), 'light': int(effect.group(2)),
             'contact': ('effect', FLUID_EFFECTS[effect.group(3)], int(effect.group(5) or 0), duration(effect.group(4)))}
    return block


def fluid_registrations():
    """One dict per `FLUIDS.register*` line of TinkerFluids, in registration order: its properties, flow family, block and bucket."""
    rows = []
    for line in FLUID_JAVA.read_text(encoding='utf-8').splitlines():
        match = re.search(r'=\s*FLUIDS\.(register\w*)\("([a-z_]+)"(.*)$', line)
        if not match:
            continue
        method, name, rest = match.groups()
        values = fluid_properties(fluid_type_expression(name, rest))
        texture = FLUID_TEXTURES / f'{name}.json'
        info = json.loads(texture.read_text(encoding='utf-8')) if texture.exists() else {}
        drop, slope, tick = FLUID_FAMILIES[method]
        override = re.search(r'\.tickRate\((\d+)\)', rest)
        base = re.search(r'\b(cool|powder|slime|hot)\(("[a-z_]*"\s*)?\)', fluid_type_expression(name, rest))
        block = fluid_block(name, rest)
        rows.append({'name': name, **values, 'color': int(info.get('color', 'FFFFFFFF'), 16), 'still': info.get('still'),
                     'drop': drop, 'slope': slope, 'tick': int(override.group(1)) if override else tick,
                     'hot': base.group(1) == 'hot', 'block': block,
                     'bucket': block is not None or 'PotionBucketItem' in rest,
                     'bucket_tagged': 'PotionBucketItem' in rest})
    return rows


def write_fluid_defs():
    """fluid_defs.rs: the original's 72 fluids with temperature, luminance, density, viscosity and tint, in TinkerFluids order, and
    how each is in the world.

    Attributes are the FluidType.Properties of each registration (the `hot`/`slime`/`cool`/`powder` helper defaults, then the
    setters), the tint is the colour of the generated mantle fluid_texture (white when it has none). The names are the
    original's `fluid.tconstruct.<name>` translations, already in the lang files. The flow is the family's of
    FluidDeferredRegisterExtension (level decrease, slope distance, tick rate), the block light and contact rule are those of
    `createBurning` / `createEffect`, and a fluid with no block (`unplacable`) has no block here: powdered snow has
    no bucket of its own either (its bucket is the vanilla item), while the potion has one, its own item, and the
    bucket carries the fluid's tag (`bucket_tagged`: the registration's `PotionBucketItem` factory)."""
    rows = fluid_registrations()
    if len(rows) != 72 or len({row['name'] for row in rows}) != 72:
        raise SystemExit(f"expected the original's 72 distinct fluids, found {len(rows)}")
    lines = ["//! The original's 72 fluids (SlimeKnights TinkerFluids, MIT; see LICENSE): attributes, tint and world behaviour, in registration order.",
             '//! Generated from the pinned original by tools/import_original_data.py; do not edit by hand.', '',
             '/// What an entity inside the fluid block suffers (BurningLiquidBlock, MobEffectLiquidBlock).',
             '#[derive(Clone, Copy, Debug, PartialEq)]',
             'pub(crate) enum Contact { None, Burning { seconds: i32, damage: f32 }, Effect { effect: &\'static str, amplifier: i32, duration: i32 } }', '',
             '/// One fluid: id path (namespace tconstruct), kelvin, luminance, density, viscosity and tint (0xAARRGGBB); whether it has a block and a bucket,',
             '/// and whether the bucket carries the fluid tag; its flow (tick delay, level drop per block, slope distance), the sounds of its',
             '/// bucket (`hot`: the lava ones), the light of its block and its contact rule.',
             "pub(crate) struct FluidSpec { pub name: &'static str, pub temperature: i32, pub luminance: u8, pub density: i32, pub viscosity: i32, pub color: u32,",
             '    pub placeable: bool, pub bucket: bool, pub bucket_tagged: bool, pub tick_delay: u16, pub level_drop: u8, pub slope_distance: u8, pub hot: bool, pub block_light: u8, pub contact: Contact }',
             '', 'pub(crate) const FLUIDS: &[FluidSpec] = &[']
    for row in rows:
        block = row['block']
        contact = 'Contact::None'
        if block and block['contact']:
            kind = block['contact']
            contact = f'Contact::Burning {{ seconds: {kind[1]}, damage: {kind[2]:.1f} }}' if kind[0] == 'burning' else f'Contact::Effect {{ effect: "{kind[1]}", amplifier: {kind[2]}, duration: {kind[3]} }}'
        lines.append(f"    FluidSpec {{ name: \"{row['name']}\", temperature: {row['temperature']}, luminance: {row['light']}, density: {row['density']}, viscosity: {row['viscosity']}, "
                     f"color: 0x{row['color']:08X}, placeable: {str(block is not None).lower()}, bucket: {str(row['bucket']).lower()}, bucket_tagged: {str(row['bucket_tagged']).lower()}, tick_delay: {row['tick']}, level_drop: {row['drop']}, "
                     f"slope_distance: {row['slope']}, hot: {str(row['hot']).lower()}, block_light: {block['light'] if block else 0}, contact: {contact} }},")
    lines.append('];')
    FLUID_DEFS_TARGET.write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
    return len(rows), [row['name'] for row in rows]


RENDER_MODEL = ROOT / '.local/tconstruct-source/src/generated/resources/assets/tconstruct/mantle/model'
RENDER_MODEL_MAIN = ROOT / '.local/tconstruct-source/src/main/resources/assets/tconstruct/mantle/model'
RENDER_DATA_TARGET = ROOT / 'tconstruct/mixins/tconstruct/api/src/render_data.rs'
RENDER_DIRECTIONS = ('down', 'up', 'north', 'south', 'west', 'east')


def _render_float(value):
    return repr(float(value))


def _render_cuboid(data):
    """One Mantle FluidCuboid as Rust: the box in sixteenths and the faces it draws (all of them when the data names none)."""
    faces = data.get('faces')
    if faces is None:
        faces = {direction: {} for direction in RENDER_DIRECTIONS}
    rendered = ', '.join(
        f"Face {{ dir: \"{direction}\", flowing: {str(bool(face.get('flowing', False))).lower()}, rotation: {int(face.get('rotation', 0))} }}"
        for direction, face in sorted(faces.items(), key=lambda item: RENDER_DIRECTIONS.index(item[0])))
    return (f"Cuboid {{ from: [{', '.join(_render_float(v) for v in data['from'])}], "
            f"to: [{', '.join(_render_float(v) for v in data['to'])}], faces: &[{rendered}] }}")


def _render_item(data):
    """One Mantle RenderItem as Rust: center and size in sixteenths, the display transform, the x/y rotation in degrees."""
    return (f"RenderItem {{ center: [{', '.join(_render_float(v) for v in data['center'])}], size: {_render_float(data['size'])}, "
            f"transform: \"{data['transform']}\", x: {_render_float(data.get('x', 0))}, y: {_render_float(data.get('y', 0))} }}")


def _render_variants(kind, name, render_one):
    """The variants of one block's entry in a mantle model folder (`block_fluids` or `item_lists`): a variant value is a
    template name (read from the folder's templates), one entry, or a list of entries."""
    data = json.loads((RENDER_MODEL / kind / f'{name}.json').read_text(encoding='utf-8'))
    variants = []
    for key, value in data['variants'].items():
        if isinstance(value, str):
            template = value.split('/')[-1]
            value = json.loads((RENDER_MODEL / kind / 'templates' / f'{template}.json').read_text(encoding='utf-8'))
        entries = value if isinstance(value, list) else [value]
        when = ', '.join(f'("{prop}", "{wanted}")' for prop, wanted in (pair.split('=', 1) for pair in key.split(','))) if key else ''
        variants.append(f"Variant {{ when: &[{when}], data: &[{', '.join(render_one(entry) for entry in entries)}] }}")
    return variants


def write_render_data():
    """render_data.rs: what the block entity renderers draw, from Mantle's model data (the fluid cuboids of
    `mantle/model/block_fluids`, the item placements of `mantle/model/item_lists`, the channel's cuboids of
    `mantle/model/channel_fluids`) and each fluid's still and flowing textures (`mantle/fluid_texture`).

    The generated block_fluids and item_lists are the data the original's datagen wrote for the resource reload; the
    channel cuboids are hand-written resources, so they are read from the source resources instead. A cuboid with no
    faces in the data draws every face; a variant key is the block state properties it asks for (empty: the default)."""
    lines = ["//! The original's block entity render data (SlimeKnights, MIT; see LICENSE): the fluid cuboids and item placements",
             '//! the block entity renderers draw, from Mantle\'s model data (`assets/tconstruct/mantle/model`), the channel\'s',
             '//! cuboids, and each fluid\'s still and flowing textures (`assets/tconstruct/mantle/fluid_texture`).',
             '//! Generated from the pinned original by tools/import_original_data.py; do not edit by hand.', '',
             '/// One face of a fluid cuboid: its direction, whether it uses the flowing texture, and the texture rotation in degrees.',
             "pub(crate) struct Face { pub dir: &'static str, pub flowing: bool, pub rotation: u16 }", '',
             '/// A fluid cuboid in sixteenths (Mantle\'s FluidCuboid): the box, and the faces it draws.',
             "pub(crate) struct Cuboid { pub from: [f32; 3], pub to: [f32; 3], pub faces: &'static [Face] }", '',
             '/// Where a block entity renderer places one item (Mantle\'s RenderItem): the center and size in sixteenths, the',
             '/// display transform, and the x and y rotation in degrees.',
             "pub(crate) struct RenderItem { pub center: [f32; 3], pub size: f32, pub transform: &'static str, pub x: f32, pub y: f32 }", '',
             '/// The data of one block state variant: the state properties it asks for (empty: the default variant).',
             "pub(crate) struct Variant<T: 'static> { pub when: &'static [(&'static str, &'static str)], pub data: &'static [T] }", '']
    for kind, function, render_one, what, type_name in (
            ('block_fluids', 'fluid_cuboids', _render_cuboid, 'fluid cuboids', 'Cuboid'),
            ('item_lists', 'render_items', _render_item, 'item placements', 'RenderItem')):
        names = sorted(path.stem for path in (RENDER_MODEL / kind).glob('*.json'))
        lines.append(f"/// The {what} of a block (`mantle/model/{kind}`), by block id.")
        lines.append(f"pub(crate) fn {function}(block: &str) -> Option<&'static [Variant<{type_name}>]> {{")
        lines.append('    Some(match block {')
        for name in names:
            variants = _render_variants(kind, name, render_one)
            lines.append(f"        \"tconstruct:{name}\" => &[")
            lines += [f'            {variant},' for variant in variants]
            lines.append('        ],')
        lines += ['        _ => return None,', '    })', '}', '']
    channel = json.loads((RENDER_MODEL_MAIN / 'channel_fluids/channel.json').read_text(encoding='utf-8'))
    lines += ['/// The channel\'s fluid cuboids (Mantle\'s ChannelFluids, `mantle/model/channel_fluids/channel.json`): the center in',
              '/// its still and flowing forms, the drop below the center, and one side (authored facing south) in its still, in,',
              '/// out and edge forms.',
              'pub(crate) struct ChannelFluids { pub center_still: Cuboid, pub center_flowing: Cuboid, pub down: Cuboid,',
              '    pub side_still: Cuboid, pub side_in: Cuboid, pub side_out: Cuboid, pub side_edge: Cuboid }',
              'pub(crate) const CHANNEL_FLUIDS: ChannelFluids = ChannelFluids {',
              f"    center_still: {_render_cuboid(channel['center']['still'])}, center_flowing: {_render_cuboid(channel['center']['flowing'])},",
              f"    down: {_render_cuboid(channel['down'])}, side_still: {_render_cuboid(channel['side']['still'])},",
              f"    side_in: {_render_cuboid(channel['side']['in'])}, side_out: {_render_cuboid(channel['side']['out'])},",
              f"    side_edge: {_render_cuboid(channel['side']['edge'])} }};", '']
    lines += ['/// A fluid\'s still and flowing textures (Mantle\'s fluid texture data, `mantle/fluid_texture`).',
              "pub(crate) struct FluidTexture { pub still: &'static str, pub flowing: &'static str }", '',
              '/// The textures of a fluid, by fluid id.',
              'pub(crate) fn fluid_texture(fluid: &str) -> Option<FluidTexture> {',
              '    Some(match fluid {']
    for path in sorted(FLUID_TEXTURES.glob('*.json')):
        info = json.loads(path.read_text(encoding='utf-8'))
        still = info.get('still')
        if still is None:
            continue
        lines.append(f"        \"tconstruct:{path.stem}\" => FluidTexture {{ still: \"{still}\", flowing: \"{info.get('flowing', still)}\" }},")
    lines += ['        _ => return None,', '    })', '}']
    RENDER_DATA_TARGET.write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
    return len(lines)


def write_fluid_world():
    """The content of the fluids that are blocks and buckets: for each fluid with a block, the block declaration (the vanilla glass template, no
    collision, a `level` state 0 to 8 and an outline only on the source, so an empty bucket can reach it), its block state and nine stepped
    models over the still texture of the original's fluid texture, and the bucket item (a plain item stacking to 1) with its model. A
    bucket without a block (the potion) gets its bucket item alone. The still textures come from the original's resources.
    Runs after copy_client_assets, which clears the folders it writes into."""

    def write_bucket(name, tinted, color):
        write_json(HORIZON_DECLARATIONS / 'item' / f'{name}_bucket.json', {'template': 'minecraft:paper', 'max_stack_size': 1}, compact=True)
        write_json(ASSET_TARGET / 'models/item' / f'{name}_bucket.json', {'parent': 'minecraft:item/generated', 'textures': {'layer0': 'minecraft:item/bucket'}})
        item = {'type': 'minecraft:model', 'model': f'tconstruct:item/{name}_bucket'}
        if tinted:
            item['tints'] = [{'type': 'minecraft:constant', 'value': color & 0xFFFFFF}]
        write_json(ASSET_TARGET / 'items' / f'{name}_bucket.json', {'model': item})

    count = 0
    for row in fluid_registrations():
        block, name = row['block'], row['name']
        if block is None:
            if row['bucket']:
                write_bucket(name, row['color'] != 0xFFFFFFFF, row['color'])
            continue
        still = row['still']
        if still is None or not still.startswith('tconstruct:'):
            raise SystemExit(f'fluid {name}: no still texture')
        path = still.split(':', 1)[1]
        source = asset_source(f'textures/{path}.png')
        if not source.exists():
            raise SystemExit(f'fluid texture missing: {still}')
        destination = ASSET_TARGET / 'textures' / f'{path}.png'
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        meta = source.parent / f'{source.name}.mcmeta'
        if meta.exists():
            (destination.parent / meta.name).write_bytes(meta.read_bytes().replace(b'\r\n', b'\n'))
        # levels 0 to 8: the source and the amounts 7 to 1 stand 8/9 to 1/9 of a block, the falling column fills it
        shapes = {str(level): ([[0, 0, 0, 16, 128 / 9, 16]] if level == 0 else []) for level in range(9)}
        properties = {'hardness': 100.0, 'explosion_resistance': 100.0, 'map_color': MAP_COLORS[block['color']], 'occludes': False}
        if block['light']:
            properties['light'] = block['light']
        tinted = row['color'] != 0xFFFFFFFF
        declaration = {'template': 'minecraft:glass', 'item': False, 'properties': properties,
                       'state_properties': {'level': {'type': 'enum', 'values': [str(level) for level in range(9)]}},
                       'default_state': {'level': '0'}, 'shapes': {'property': 'level', 'values': shapes}, 'collision_shapes': {'default': []}}
        write_json(HORIZON_DECLARATIONS / 'block' / f'{name}_fluid.json', declaration, compact=True)
        variants = {}
        for level in range(9):
            amount = 8 if level in (0, 8) else 8 - level
            height = 16 if level == 8 else amount * 16 / 9
            face = {'texture': '#fluid', **({'tintindex': 0} if tinted else {})}
            model = {'ambientocclusion': False, 'textures': {'fluid': still, 'particle': still},
                     'elements': [{'from': [0, 0, 0], 'to': [16, round(height, 4), 16], 'faces': {side: dict(face) for side in ('down', 'up', 'north', 'south', 'west', 'east')}}]}
            write_json(ASSET_TARGET / 'models/block/fluid' / name / f'level_{level}.json', model)
            variants[f'level={level}'] = {'model': f'tconstruct:block/fluid/{name}/level_{level}'}
        write_json(ASSET_TARGET / 'blockstates' / f'{name}_fluid.json', {'variants': variants})
        write_bucket(name, tinted, row['color'])
        count += 1
    return count


def copy_fluid_textures():
    """The whole fluid texture tree of the original (textures/fluid), byte for byte: write_fluid_world copies
    only the still texture of each fluid with a block, but the renderers also name every fluid's flowing
    texture (render_data.rs), the two gas fluids' inverted flowing textures (Mantle's convention for a fluid
    lighter than air: cinderslime and ichor), and the stills of the fluids without a block (potion, powdered
    snow). PNGs are copied raw and .mcmeta files with their line endings normalised, as write_fluid_world
    does for the stills. Left out: molten/shiny and molten/transparent, which nothing in the original names
    (no fluid texture data, model or code reaches them) — dead files the content census forbids shipping.
    Runs after copy_client_assets, which clears the folders it writes into. Returns the files written."""
    source_root = asset_source('textures/fluid')
    target_root = ASSET_TARGET / 'textures/fluid'
    written = 0
    for source in sorted(source_root.rglob('*')):
        if not source.is_file():
            continue
        relative = source.relative_to(source_root)
        if relative.parts[:2] in (('molten', 'shiny'), ('molten', 'transparent')):
            continue
        destination = target_root / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        if source.suffix == '.mcmeta':
            destination.write_bytes(source.read_bytes().replace(b'\r\n', b'\n'))
        else:
            shutil.copyfile(source, destination)
        written += 1
    return written


def copy_modifier_icons():
    """The modifier icon set, byte for byte: assets/tconstruct/tinkering/modifier_icons.json (the icon
    manager's per-modifier overrides: icon sprite and book cover per modifier) and the whole
    textures/gui/modifiers tree the manager names by convention (gui/modifiers/<category>/<modifier>,
    with the top-level sprites as the shared covers and fallbacks). Nothing reaches these through
    models - the worktable and book screens draw them by modifier id - so, like the fluid textures,
    they are copied as a whole tree rather than by reachability. The JSON's line endings are
    normalised, as elsewhere for data text. Runs after copy_client_assets."""
    source = asset_source('tinkering/modifier_icons.json')
    destination = ASSET_TARGET / 'tinkering/modifier_icons.json'
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(source.read_bytes().replace(b'\r\n', b'\n'))
    source_root = asset_source('textures/gui/modifiers')
    target_root = ASSET_TARGET / 'textures/gui/modifiers'
    for source in sorted(source_root.rglob('*')):
        if not source.is_file():
            continue
        destination = target_root / source.relative_to(source_root)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)


def copy_trim_palettes():
    """The trim colour palettes, byte for byte: textures/trims/color_palettes of the original's generated
    resources, one PNG per trim material this port ships as data (data/tconstruct/trim_material): the
    material's asset_name (tconstruct_<name>) names its palette by the trim convention
    (textures/trims/color_palettes/<name>.png), the sixteen names matching one for one. Nothing reaches
    them through models - the trim renderer reads the palette texture by that path - so, like the modifier
    icons, they are copied as a whole tree rather than by reachability. The trim rendering consumer is a
    separate modifier-side gap; the data side ships complete. Runs after copy_client_assets."""
    source_root = asset_source('textures/trims/color_palettes')
    target_root = ASSET_TARGET / 'textures/trims/color_palettes'
    written = 0
    for source in sorted(source_root.glob('*.png')):
        destination = target_root / source.name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        written += 1
    return written


def copy_colormaps():
    """The slime grass colormaps, byte for byte: textures/colormap of the original (blood, earth, ender,
    ichor, sky and plain slime grass), sampled by the client colour providers per slime grass type - the
    six grass types all ship in this port, and the resource facade is expected to read the actual files
    (the PlutoLib probe names earth_grass_color.png). Nothing reaches them through models, so, like the
    modifier icons, they are copied as a whole tree rather than by reachability. Runs after
    copy_client_assets."""
    source_root = asset_source('textures/colormap')
    target_root = ASSET_TARGET / 'textures/colormap'
    written = 0
    for source in sorted(source_root.glob('*.png')):
        destination = target_root / source.name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        written += 1
    return written


def copy_mob_effect_icons():
    """The mob effect icons, byte for byte: the whole textures/mob_effect tree of the original. The host
    resolves an effect's icon by convention - horizon-client-core's potions.rs builds the sprite name
    mob_effect/<path> for vanilla effects and <ns>:mob_effect/<path> for a mod's, and the HUD stitches
    assets/<ns>/textures/mob_effect into its icon atlas - so the shipped effects' icons (including the
    carry amplifier variants and the helmet charging bar) are named by effect id alone, with no model or
    data field pointing at them. Like the modifier icons, the tree is copied whole rather than by
    reachability. Runs after copy_client_assets."""
    source_root = asset_source('textures/mob_effect')
    target_root = ASSET_TARGET / 'textures/mob_effect'
    written = 0
    for source in sorted(source_root.glob('*.png')):
        destination = target_root / source.name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        written += 1
    return written


def write_fluid_atlas():
    """The fluid directory's atlas sources (assets/minecraft/atlases/blocks.json): the game's unified atlas
    (screen draws and native block-entity draws) packs the walked texture folders and the base game's atlas
    definitions, which name only block/ and entity/conduit/, so textures/fluid/ would never be stitched and
    every fluid face the renderers draw would be skipped as an unknown sprite. A content mod's
    assets/minecraft/atlases/*.json is not copied by the host's builder: its sources are appended to the
    base definition of the same name, so this declaration adds every namespace's fluid directory to the
    blocks atlas under the sprites' own ids (tconstruct:fluid/...)."""
    write_json(ROOT / 'tconstruct/content/assets/minecraft/atlases/blocks.json',
               {'sources': [{'type': 'minecraft:directory', 'source': 'fluid', 'prefix': 'fluid/'}]})


def write_copper_can():
    """The copper can's item model: the original's own fluid container loader is not available, so the can shows its base texture
    (the fluid layer of the original's model is not drawn). Runs after copy_client_assets, which clears the folders it writes into."""
    source = asset_source('textures/item/copper_can.png')
    destination = ASSET_TARGET / 'textures/item/copper_can.png'
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)
    write_json(ASSET_TARGET / 'models/item/copper_can.json', {'parent': 'minecraft:item/generated', 'textures': {'layer0': 'tconstruct:item/copper_can'}})
    write_json(ASSET_TARGET / 'items/copper_can.json', {'model': {'type': 'minecraft:model', 'model': 'tconstruct:item/copper_can'}})


FLUID_TRANSFER_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/mantle/fluid_transfer'
FLUID_TRANSFER_TARGET = ROOT / 'tconstruct/mixins/tconstruct/api/src/fluid_transfers.rs'
FLUID_TAG_FOLDERS = [ROOT / '.local/tconstruct-source/src/generated/resources/data/forge/tags/fluids', ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/tags/fluids']


def fluid_tag_members(tag, seen=()):
    """The source fluids of a fluid tag of the original: references followed, flowing variants dropped, other namespaces left out."""
    namespace, path = tag.split(':', 1)
    folder = {'forge': FLUID_TAG_FOLDERS[0], 'tconstruct': FLUID_TAG_FOLDERS[1]}[namespace]
    found = []
    for entry in json.loads((folder / f'{path}.json').read_text(encoding='utf-8'))['values']:
        entry = entry if isinstance(entry, str) else entry['id']
        if entry.startswith('#'):
            if entry[1:] not in seen:
                found += fluid_tag_members(entry[1:], seen + (tag,))
        elif entry.startswith('tconstruct:') and not entry.startswith('tconstruct:flowing_'):
            found.append(entry.split(':', 1)[1])
    return found


def write_fluid_transfers():
    """fluid_transfers.rs: the original's `mantle:fill_item` and `mantle:empty_item` fluid transfers of Tinkers (a bowl or glass bottle fills
    with 250 of a fluid, the meat soup and the bottles empty), fluid tags resolved to the source fluids they name. The two transfers that need
    Twilight Forest items are left out (other-mod integration)."""
    rows = []
    for path in sorted(FLUID_TRANSFER_SOURCE.glob('*.json')):
        data = json.loads(path.read_text(encoding='utf-8'))
        if 'conditions' in data:
            continue
        kind = {'mantle:fill_item': 'Fill', 'mantle:empty_item': 'Empty'}[data['type']]
        fluid = data['fluid']
        names = [fluid['fluid'].split(':', 1)[1]] if 'fluid' in fluid else fluid_tag_members(fluid['tag'])
        item = data['input']['item']
        result = data['result']
        rows.append((path.stem, kind, item, names, fluid['amount'], result))
    lines = ["//! The original's Tinkers fluid container transfers (Mantle `fill_item` / `empty_item`, SlimeKnights; see LICENSE): a bowl or glass bottle fills with",
             '//! an amount of a fluid and becomes the filled item, a filled item gives the amount back as the empty one. Generated from the pinned original',
             '//! by tools/import_original_data.py; do not edit by hand.', '',
             '#[derive(Clone, Copy, Debug, PartialEq, Eq)]', 'pub(crate) enum Kind { Fill, Empty }', '',
             '/// One transfer: its name, direction, the held item, the fluids (ids in the tconstruct namespace) it takes or gives, the amount and the resulting item.',
             'pub(crate) struct Transfer { pub name: &\'static str, pub kind: Kind, pub item: &\'static str, pub fluids: &\'static [&\'static str], pub amount: u32, pub result: &\'static str }', '',
             'pub(crate) const TRANSFERS: &[Transfer] = &[']
    for name, kind, item, names, amount, result in rows:
        fluids = ', '.join(f'"{n}"' for n in names)
        lines.append(f'    Transfer {{ name: "{name}", kind: Kind::{kind}, item: "{item}", fluids: &[{fluids}], amount: {amount}, result: "{result}" }},')
    lines.append('];')
    FLUID_TRANSFER_TARGET.write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
    return len(rows)


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


VANILLA_TAGS = ROOT / '.local/base-26.3/data'


def write_block_tag_ingredients():
    """Item tags for the `tconstruct:block_tag` ingredients of the recipes (a casting cast "any block of this block tag", such as
    `minecraft:convertable_to_mud`): the host snapshots item tags only, so the block tag becomes an item tag `block_tags/<ns>/<path>` of the
    blocks' items (a block item has its block's id), flattened from the vanilla tag files, every entry optional. The original's 1.20 spelling
    `convertable_to_mud` is vanilla's `convertible_to_mud` here. The Rust decoder reads the ingredient as that item tag."""
    names = set()
    for path in FLUID_RECIPE_SOURCE.rglob('*.json'):
        for match in re.finditer(r'"type": "tconstruct:block_tag",\s*"tag": "([a-z_:/]+)"', path.read_text(encoding='utf-8')):
            names.add(match.group(1))

    def members(tag, seen=()):
        namespace, name = tag.split(':', 1)
        file = VANILLA_TAGS / namespace / 'tags/block' / f'{name}.json'
        if not file.is_file() and 'convertable' in name:
            file = VANILLA_TAGS / namespace / 'tags/block' / f'{name.replace("convertable", "convertible")}.json'
        if not file.is_file() or tag in seen:
            raise SystemExit(f'block tag {tag} is not in the vanilla data')
        found = []
        for entry in json.loads(file.read_text(encoding='utf-8')).get('values', []):
            text = entry if isinstance(entry, str) else entry['id']
            for member in (members(text[1:], seen + (tag,)) if text.startswith('#') else [text]):
                if member not in found:
                    found.append(member)
        return found

    for tag in sorted(names):
        namespace, name = tag.split(':', 1)
        write_json(TAG_TARGET / 'item/block_tags' / namespace / f'{name}.json', {'values': [{'id': member, 'required': False} for member in members(tag)]})
    return len(names)


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
    # is_freezing is the source frostshield's damage_to_capacity names. bypasses_effects and
    # bypasses_invulnerability are wetting's skip gates, avoids_guardian_thorns the directness
    # gate bursting shares with it (both read from the pure layer's DAMAGE_TYPE_TAGS too).
    wanted = {'bypasses_invulnerability', 'bypasses_enchantments', 'is_freezing', 'bypasses_effects', 'avoids_guardian_thorns'}
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


def write_instrument_tags():
    """instrument_tags.rs: the pinned vanilla instrument registry and the instrument tags, flattened.

    The host captures no instrument registry, so the `mantle:tag_combination_filled` conditions on the
    instrument registry and the `tconstruct:instrument` ingredients read this table instead. The registry
    is the vanilla data's instrument definitions; the tags are vanilla's (from the pinned vanilla data) and
    the original's own (from its generated resources), with `#tag` references flattened."""
    registry = sorted(f'minecraft:{path.stem}' for path in (ROOT / '.local/base-26.3/data/minecraft/instrument').glob('*.json'))
    files = {}
    for path in sorted((VANILLA_TAGS / 'minecraft/tags/instrument').glob('*.json')):
        files[f'minecraft:{path.stem}'] = path
    for path in sorted((TAG_SOURCE / 'instrument').glob('*.json')):
        files[f'tconstruct:{path.stem}'] = path

    def flatten(tag, seen=()):
        if tag in seen or tag not in files:
            raise SystemExit(f'instrument tag {tag} is not in the vanilla or original data')
        members = []
        for value in json.loads(files[tag].read_text(encoding='utf-8')).get('values', []):
            text = value if isinstance(value, str) else value['id']
            for member in (flatten(text[1:], seen + (tag,)) if text.startswith('#') else [text]):
                if member not in members:
                    members.append(member)
        return members

    lines = ['//! The pinned vanilla instrument registry and the instrument tags the shipped data names, flattened.',
             '//! Generated from the pinned vanilla data and the original\'s generated resources by tools/import_original_data.py; do not edit by hand.', '',
             'pub(crate) const REGISTRY: &[&str] = &[' + ', '.join(f'"{name}"' for name in registry) + '];', '',
             'pub(crate) const TAGS: &[(&str, &[&str])] = &[']
    for tag in sorted(files):
        lines.append(f'    ("{tag}", &[' + ', '.join(f'"{member}"' for member in flatten(tag)) + ']),')
    lines.append('];')
    (ROOT / 'tconstruct/mixins/tconstruct/api/src/instrument_tags.rs').write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')


# Forge's own convention item tags (the original's recipes name them; nothing in this host defines them): the vanilla items each stands for,
# as Forge defines them. The original's own contributions (generated `data/forge/tags/items`: glass, normal_stone, the Tinkers metals...) are
# added, reduced to vanilla items and to the Tinkers items this port declares. The result is applied to the item registry snapshot by
# convention_tags.rs, and replaces `#forge:` entries of the shipped item tags.
def _vanilla(*names):
    return [f'minecraft:{name}' for name in names]


VANILLA_CONVENTION = {
    'ingots/iron': _vanilla('iron_ingot'), 'ingots/gold': _vanilla('gold_ingot'), 'ingots/copper': _vanilla('copper_ingot'),
    'ingots/netherite': _vanilla('netherite_ingot'), 'ingots/netherite_scrap': _vanilla('netherite_scrap'),
    'nuggets/iron': _vanilla('iron_nugget'), 'nuggets/gold': _vanilla('gold_nugget'),
    'storage_blocks/iron': _vanilla('iron_block'), 'storage_blocks/gold': _vanilla('gold_block'), 'storage_blocks/copper': _vanilla('copper_block'),
    'storage_blocks/diamond': _vanilla('diamond_block'), 'storage_blocks/emerald': _vanilla('emerald_block'),
    'storage_blocks/netherite': _vanilla('netherite_block'), 'storage_blocks/quartz': _vanilla('quartz_block'),
    'storage_blocks/lapis': _vanilla('lapis_block'), 'storage_blocks/redstone': _vanilla('redstone_block'),
    'storage_blocks/coal': _vanilla('coal_block'), 'storage_blocks/amethyst': _vanilla('amethyst_block'),
    'storage_blocks/raw_iron': _vanilla('raw_iron_block'), 'storage_blocks/raw_gold': _vanilla('raw_gold_block'), 'storage_blocks/raw_copper': _vanilla('raw_copper_block'),
    'gems/diamond': _vanilla('diamond'), 'gems/emerald': _vanilla('emerald'), 'gems/quartz': _vanilla('quartz'),
    'gems/amethyst': _vanilla('amethyst_shard'), 'gems/lapis': _vanilla('lapis_lazuli'),
    'raw_materials/iron': _vanilla('raw_iron'), 'raw_materials/gold': _vanilla('raw_gold'), 'raw_materials/copper': _vanilla('raw_copper'),
    'ores/iron': _vanilla('iron_ore', 'deepslate_iron_ore'), 'ores/gold': _vanilla('gold_ore', 'deepslate_gold_ore', 'nether_gold_ore'),
    'ores/copper': _vanilla('copper_ore', 'deepslate_copper_ore'), 'ores/diamond': _vanilla('diamond_ore', 'deepslate_diamond_ore'),
    'ores/emerald': _vanilla('emerald_ore', 'deepslate_emerald_ore'), 'ores/lapis': _vanilla('lapis_ore', 'deepslate_lapis_ore'),
    'ores/redstone': _vanilla('redstone_ore', 'deepslate_redstone_ore'), 'ores/coal': _vanilla('coal_ore', 'deepslate_coal_ore'),
    'ores/quartz': _vanilla('nether_quartz_ore'), 'ores/netherite_scrap': _vanilla('ancient_debris'),
    'dusts/redstone': _vanilla('redstone'), 'dusts/glowstone': _vanilla('glowstone_dust'),
    'obsidian': _vanilla('obsidian'), 'cobblestone': _vanilla('cobblestone', 'mossy_cobblestone', 'cobbled_deepslate'),
    'stone': _vanilla('stone'), 'netherrack': _vanilla('netherrack'), 'end_stones': _vanilla('end_stone'),
    'ender_pearls': _vanilla('ender_pearl'), 'feathers': _vanilla('feather'), 'leather': _vanilla('leather'), 'gunpowder': _vanilla('gunpowder'),
    'bones': _vanilla('bone'), 'slimeballs': _vanilla('slime_ball'), 'string': _vanilla('string'), 'rods/wooden': _vanilla('stick'),
    'rods/blaze': _vanilla('blaze_rod'), 'sand': _vanilla('sand', 'red_sand'), 'sand/colorless': _vanilla('sand'), 'sand/red': _vanilla('red_sand'),
    'crops/carrot': _vanilla('carrot'), 'bottles/empty': _vanilla('glass_bottle'),
}
_COLORS = ['white', 'orange', 'magenta', 'light_blue', 'yellow', 'lime', 'pink', 'gray', 'light_gray', 'cyan', 'purple', 'blue', 'brown', 'green', 'red', 'black']
VANILLA_CONVENTION.update({
    'glass/colorless': _vanilla('glass'), 'glass/tinted': _vanilla('tinted_glass'),
    'glass/silica': _vanilla('glass', *[f'{color}_stained_glass' for color in _COLORS]),
    'glass': _vanilla('glass', 'tinted_glass', *[f'{color}_stained_glass' for color in _COLORS]),
    'glass_panes/colorless': _vanilla('glass_pane'), 'glass_panes/silica': _vanilla('glass_pane', *[f'{color}_stained_glass_pane' for color in _COLORS]),
    'glass_panes': _vanilla('glass_pane', *[f'{color}_stained_glass_pane' for color in _COLORS]),
    **{f'glass/{color}': _vanilla(f'{color}_stained_glass') for color in _COLORS},
    **{f'glass_panes/{color}': _vanilla(f'{color}_stained_glass_pane') for color in _COLORS},
})
CONVENTION_ROLLUPS = {'rods': 'rods/', 'ingots': 'ingots/', 'nuggets': 'nuggets/', 'gems': 'gems/', 'ores': 'ores/', 'raw_materials': 'raw_materials/', 'storage_blocks': 'storage_blocks/'}
CONVENTION_TARGET = ROOT / 'tconstruct/mixins/tconstruct/api/src/convention_tags.rs'


def convention_tags():
    """forge tag path -> sorted members (vanilla and declared Tinkers items), for every convention tag with at least one."""
    declared = {f'tconstruct:{p.stem}' for kind in ('block', 'item') for p in (HORIZON_DECLARATIONS / kind).glob('*.json')}
    folder = GENERATED_DATA / 'forge/tags/items'
    result = {tag: list(members) for tag, members in VANILLA_CONVENTION.items()}

    def add(tag, members):
        held = result.setdefault(tag, [])
        held += [member for member in members if member not in held]

    def flatten(tag, seen=()):
        name = tag[len('forge:'):]
        path = folder / f'{name}.json'
        if tag in seen or not path.is_file():
            return list(result.get(name, []))
        out = []
        for entry in json.loads(path.read_text(encoding='utf-8')).get('values', []):
            text = entry if isinstance(entry, str) else entry['id']
            if text.startswith('#'):
                if text.startswith('#forge:'):
                    out += flatten(text[1:], seen + (tag,))
            elif text.startswith('minecraft:') or text in declared:
                out.append(text)
        return out

    for path in sorted(folder.rglob('*.json')):
        name = path.relative_to(folder).with_suffix('').as_posix()
        add(name, flatten('forge:' + name))
    for roll, prefix in CONVENTION_ROLLUPS.items():
        for tag in sorted(result):
            if tag.startswith(prefix):
                add(roll, result[tag])
    return {tag: sorted(members) for tag, members in sorted(result.items()) if members}


_CONVENTION = {}


def convention_tags_cached():
    if not _CONVENTION:
        _CONVENTION.update(convention_tags())
    return _CONVENTION


def write_convention_tags():
    tags = convention_tags_cached()
    lines = ['//! Forge convention item tags that the original data names and nothing in this host defines, with the vanilla and Tinkers items they stand for.',
             '//! Generated from the original data by tools/import_original_data.py; do not edit by hand.', '',
             'pub(crate) const TAGS: &[(&str, &[&str])] = &[']
    for tag, members in tags.items():
        lines.append(f'    ("forge:{tag}", &[' + ', '.join(f'"{member}"' for member in members) + ']),')
    lines.append('];')
    CONVENTION_TARGET.write_text(chr(10).join(lines) + chr(10), encoding='utf-8', newline=chr(10))
    return len(tags)


def registered_entity_types():
    """The Tinkers entity types that exist in this port: the slime mobs and the gadget entities registered
    from Rust (slime_entities.rs, fancy_gadgets.rs) and the projectile types the content declares."""
    declared = {path.stem for path in (HORIZON_DECLARATIONS / 'entity_type').glob('*.json')}
    return {*SLIME_ENTITIES, 'fancy_item_frame', 'armor_stand', *declared}


def copy_tags():
    """Item, block, entity-type, block-entity-type, potion and menu tags. Forge's and Mantle's tag
    namespaces do not exist in this host, so a required reference to one of their tags becomes an
    optional one (as the original already does for other mods' entries); everything else is kept in
    order, unchanged."""
    if TAG_TARGET.exists():
        shutil.rmtree(TAG_TARGET)
    count = 0
    for source_name, target_name in TAG_FOLDER_NAMES.items():
        for path in sorted((TAG_SOURCE / source_name).rglob('*.json')):
            data = json.loads(path.read_text(encoding='utf-8'))
            values = []
            for value in data.get('values', []):
                text = value if isinstance(value, str) else value.get('id', '')
                if source_name == 'items' and text.startswith('#forge:') and convention_tags_cached().get(text[len('#forge:'):]):
                    # a convention tag this host lacks: the items it stands for, in its place
                    held = {(entry if isinstance(entry, str) else entry['id']) for entry in values}
                    values += [{'id': member, 'required': False} for member in convention_tags_cached()[text[len('#forge:'):]] if member not in held]
                    continue
                if isinstance(value, str) and text.lstrip('#').split(':')[0] in ('forge', 'mantle'):
                    value = {'id': text, 'required': False}
                # An entity-type tag naming a Tinkers entity type this port has not registered would make the
                # whole tag capture unusable for PlutoLib (an unknown member), so it waits for its entity; the
                # registered ones ship (registered_entity_types).
                if source_name == 'entity_types' and isinstance(value, str) and text.startswith('tconstruct:') and text[len('tconstruct:'):] not in registered_entity_types():
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
# Smeltery blocks whose block states are the original's own (facing, active, in_structure), and the GUI textures their screens draw.
SMELTERY_STATE_BLOCKS = ['seared_melter', 'seared_heater', 'smeltery_controller', 'foundry_controller', 'scorched_alloyer'] + [f'{kind}_{block}' for kind in ('seared', 'scorched') for block in ('table', 'basin', 'faucet', 'channel')]
SMELTERY_GUI_TEXTURES = ['melter', 'heater', 'heating_structure', 'duct', 'alloyer']
# Station screens' textures and the pattern sprites their windows draw (station_layouts' sprite table).
STATION_GUI_TEXTURES = ['tinker', 'part_builder', 'icons']
STATION_PATTERN_SPRITES = ['adze_head', 'arrow_head', 'arrow_part', 'arrow_shaft', 'block', 'boots_plating',
    'bow_grip', 'bow_limb', 'bowstring', 'broad_axe_head', 'broad_blade', 'button_repair', 'chestplate_plating',
    'coin', 'crystal', 'dust', 'fletching', 'gear', 'gem', 'hammer_head', 'helmet_plating', 'ingot', 'laces',
    'lapis', 'large_plate', 'leggings_plating', 'maille', 'nugget', 'pick_head', 'pickaxe', 'plate', 'plate_armor',
    'plating', 'quartz', 'repair_kit', 'ribcage', 'rod', 'scale', 'shard', 'shell', 'shield_core',
    'small_axe_head', 'small_blade', 'thrown_ammo', 'tool_binding', 'tool_handle', 'tough_binding',
    'tough_handle', 'wire']
# Loaders that only retexture an otherwise ordinary model: dropped, the model keeps its shape.
# tconstruct:tank only adds the fluid drawn inside an ordinary element model: the shape stays, the fluid is a renderer's work.
# tconstruct:fluid_texture (the heating controllers' window) draws the smeltery's fluid in a window element: the shape stays, the fluid is a renderer's work.
# `mantle:item_layer` and `mantle:colored_block` carry nothing but a luminosity hint beyond the
# textures, so they flatten like the rest; the hint is dropped.
# tconstruct:material_block (the fake storage block) picks the cube's texture from the block's material: the shape stays
# with its fallback texture, the material texture is a renderer's work.
PLAIN_LOADERS = {'mantle:retextured', 'tconstruct:tank', 'tconstruct:fluid_texture', 'mantle:connected', 'mantle:item_layer', 'mantle:colored_block', 'tconstruct:material_block'}


def qualify(reference):
    return reference if ':' in reference else f'minecraft:{reference}'


def asset_source(relative):
    """An original client asset: the hand-written ones are in src/main, the data-generated ones (item models of plain
    items, block states of generated blocks) in src/generated. A name is in at most one place; src/main wins."""
    for base in (ASSET_SOURCE, GENERATED_ASSETS):
        if (base / relative).exists():
            return base / relative
    return ASSET_SOURCE / relative


def gadget_items():
    """The fancy item frames and armor stands of TinkerGadgets, as item declarations.

    In the original these are items whose use places a hanging or standing entity (the item classes copy the
    vanilla placement logic of ItemFrameItem/ArmorStandItem; the variant behaviour lives in the entity classes).
    The declaration templates are therefore the vanilla items themselves: the frame items are registered with
    plain item properties (stack 64, TinkerGadgets ITEM_PROPS) and the stand items with stacksTo(16). The
    derived entity types the original registers for the variants (tconstruct:fancy_item_frame,
    tconstruct:armor_stand) are registered from Rust (api/src/fancy_gadgets.rs); the host has no spawn reach or
    simulation callbacks for derived types, so a placed item currently yields the vanilla entity. The frame
    block models and stand item models and textures are pulled in by copy_client_assets like any other item."""
    frames = {f'{name}_item_frame': {'template': 'minecraft:item_frame'} for name in
              ('gold', 'reversed_gold', 'diamond', 'manyullyn', 'netherite', 'clear')}
    stands = {f'{name}_armor_stand': {'template': 'minecraft:armor_stand', 'max_stack_size': 16} for name in
              ('bamboo', 'bone', 'necrotic_bone', 'clear')}
    return {**frames, **stands}


def write_gadget_declarations():
    for name, declaration in gadget_items().items():
        (HORIZON_DECLARATIONS / 'item' / f'{name}.json').write_text(json.dumps(declaration, indent=2) + '\n', encoding='utf-8', newline='\n')


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
                # only the line endings are normalized (the original checkout may be CRLF)
                (destination.parent / meta.name).write_bytes(meta.read_bytes().replace(b'\r\n', b'\n'))
        return reference

    def import_model(reference):
        reference = qualify(reference)
        # Forge's two item parents are the vanilla generated and handheld models under another
        # name; the tool item definitions already map them so (FORGE_PARENTS, below).
        reference = FORGE_PARENTS.get(reference, reference)
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
        source_model = read(asset_source(f'models/{path}.json'))
        converted = {}
        for key, value in source_model.items():
            # render_type: 26.x derives the render layer from the texture's alpha (its own slime block model has none).
            if key in ('loader', 'retextured', '__comment', 'render_type', 'colors', 'fluids'):
                continue
            if key == 'parent':
                value = import_model(value)
            elif key == 'textures':
                value = {name: texture if texture.startswith('#') else import_texture(texture) for name, texture in value.items()}
            elif key == 'elements':
                value = [{k: v for k, v in element.items() if k != '__comment'} for element in value]
            converted[key] = value
        if 'connection' in source_model:
            # Mantle's connected-texture set of a glass texture: the direction-combination sprites (and the
            # smeltery and foundry folders' `normal`) in the texture's own folder, picked at render time by
            # the neighbours' connections; they ship with the model whose connection data names the texture.
            for texture in source_model.get('textures', {}).values():
                if texture.startswith('#'):
                    continue
                folder_path = qualify(texture).split(':', 1)[1]
                folder = asset_source(f'textures/{folder_path}')
                if folder.is_dir():
                    for sprite in sorted(folder.glob('*.png')):
                        import_texture(f'tconstruct:{folder_path}/{sprite.stem}')
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
        blockstate = read(asset_source(f'blockstates/{name}.json'))
        variants = blockstate.get('variants', {})
        if (name in SMELTERY_STATE_BLOCKS or name in WORLD_STATE_BLOCKS) and 'multipart' in blockstate:
            # the channel's parts: each applies a model (or a list of them) when its states hold
            parts = blockstate['multipart']
            applies = [apply for part in parts for apply in (part['apply'] if isinstance(part['apply'], list) else [part['apply']])]
            if any(needs_loader(apply['model']) for apply in applies):
                skipped.append(name)
                continue
            converted = [{**part, 'apply': ([{**a, 'model': import_model(a['model'])} for a in part['apply']] if isinstance(part['apply'], list) else {**part['apply'], 'model': import_model(part['apply']['model'])})} for part in parts]
            write(f'blockstates/{name}.json', {'multipart': converted})
            write(f'items/{name}.json', {'model': {'type': 'minecraft:model', 'model': import_model(f'tconstruct:item/{name}')}})
            continue
        if name in SMELTERY_STATE_BLOCKS or name in WORLD_STATE_BLOCKS:
            # The controllers declare facing/active/in_structure themselves, so the original's variants apply as they are.
            if any(needs_loader(variant['model']) for variant in variants.values()):
                skipped.append(name)
                continue
            converted = {key: {**variant, 'model': import_model(variant['model'])} for key, variant in variants.items()}
            write(f'blockstates/{name}.json', {'variants': converted})
            first = converted[next(iter(converted))]
            item_model = asset_source(f'models/item/{name}.json').exists()
            write(f'items/{name}.json', {'model': {'type': 'minecraft:model', 'model': import_model(f'tconstruct:item/{name}') if item_model else first['model']}})
            continue
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
    for texture in SMELTERY_GUI_TEXTURES:
        import_texture(f'tconstruct:gui/{texture}')
    for texture in STATION_GUI_TEXTURES:
        import_texture(f'tconstruct:gui/{texture}')
    for sprite in STATION_PATTERN_SPRITES:
        import_texture(f'tconstruct:gui/tinker_pattern/{sprite}')
    for name in sorted(path.stem for path in (HORIZON_DECLARATIONS / 'item').glob('*.json')):
        if name.endswith('_spawn_egg'):
            # The original's egg model is the vanilla tint template, which 26.3 dropped for per-egg textures the
            # original does not ship; the slime eggs stay unmodelled (their declarations are write_slime_entities').
            skipped.append(name)
            continue
        if not asset_source(f'models/item/{name}.json').exists():
            skipped.append(name)
            continue
        if read(asset_source(f'models/item/{name}.json')).get('loader') == 'mantle:nbt_key':
            # The crystalshot: its texture is chosen by the stack's `variant` data, which no item-model select
            # here can read; the default texture stands in and the variant textures are left out.
            model = read(asset_source(f'models/item/{name}.json'))
            write(f'models/item/{name}.json', {'parent': 'minecraft:item/generated', 'textures': {'layer0': import_texture(model['textures']['default'])}})
            write(f'items/{name}.json', {'model': {'type': 'minecraft:model', 'model': f'tconstruct:item/{name}'}})
            continue
        if needs_loader(f'tconstruct:item/{name}'):
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


# Tool definitions are named from family and slot, so the slime chestplate's definition is
# `slime_chestplate`; the original registers (and models) its item as `slimy_chestplate`.
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
        write_json(ASSET_TARGET / 'items' / f'{MODEL_ALIASES.get(name, name)}.json', {'model': full}, compact=True)
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
MAIN_DATA = ROOT / '.local/tconstruct-source/src/main/resources/data'
CONTENT_DATA = ROOT / 'tconstruct/content/data'
# MapColor ids (0 to 61) as the host reads them; each one's colour was checked against the host's MAP_COLORS table.
MAP_COLOR = {'NONE': 0, 'GRASS': 1, 'METAL': 6, 'PLANT': 7, 'SNOW': 8, 'STONE': 11, 'COLOR_ORANGE': 15, 'COLOR_MAGENTA': 16, 'COLOR_LIGHT_BLUE': 17, 'COLOR_YELLOW': 18, 'COLOR_LIGHT_GREEN': 19,
             'COLOR_PINK': 20, 'COLOR_GRAY': 21, 'COLOR_LIGHT_GRAY': 22, 'COLOR_CYAN': 23, 'COLOR_PURPLE': 24, 'COLOR_BLUE': 25, 'COLOR_BROWN': 26, 'COLOR_GREEN': 27,
             'COLOR_RED': 28, 'COLOR_BLACK': 29, 'GOLD': 30, 'DIAMOND': 31, 'NETHER': 35, 'TERRACOTTA_WHITE': 36, 'TERRACOTTA_ORANGE': 37, 'TERRACOTTA_LIGHT_BLUE': 39,
             'TERRACOTTA_LIGHT_GRAY': 44, 'TERRACOTTA_CYAN': 45, 'TERRACOTTA_BLUE': 47, 'TERRACOTTA_BROWN': 48, 'TERRACOTTA_RED': 50, 'WARPED_NYLIUM': 55, 'WARPED_STEM': 56, 'WARPED_WART_BLOCK': 58}
# SlimeType: map colour and light level. DirtType.TINKER: map colour.
SLIME_TYPES = {'earth': ('GRASS', 0), 'sky': ('DIAMOND', 0), 'ichor': ('COLOR_ORANGE', 10), 'ender': ('COLOR_PURPLE', 0)}
DIRT_TYPES = {'earth': 'GRASS', 'sky': 'WARPED_STEM', 'ichor': 'TERRACOTTA_LIGHT_BLUE', 'ender': 'TERRACOTTA_ORANGE'}
# Plain items: TinkerWorld's with Item.Properties (TooltipItem only adds the lang tooltip text) and TinkerCommons' tinker slime
# balls (`new Item(ITEM_PROPS)` for SlimeType.TINKER), which the congealed slime blocks drop.
WORLD_ITEMS = ['seared_brick', 'scorched_brick', 'raw_cobalt', 'cobalt_shard', 'steel_shard', 'knightmetal_shard', 'sky_slime_ball', 'ichor_slime_ball', 'ender_slime_ball',
               'earth_slime_crystal', 'sky_slime_crystal', 'ichor_slime_crystal', 'ender_slime_crystal',
               'earth_slime_grass_seeds', 'sky_slime_grass_seeds', 'ichor_slime_grass_seeds', 'ender_slime_grass_seeds', 'blood_slime_grass_seeds']
# The shipped metals of TinkerMaterials (`registerMetal` each: a storage block, an ingot and a nugget; soulsteel
# has no compaction recipes, but is registered all the same). Their items are plain items.
METALS = ['cobalt', 'steel', 'slimesteel', 'amethyst_bronze', 'rose_gold', 'pig_iron', 'cinderslime', 'queens_slime', 'manyullyn', 'hepatizon', 'knightmetal', 'knightslime', 'soulsteel']
METAL_ITEMS = [f'{metal}_{part}' for metal in METALS for part in ('ingot', 'nugget')] + ['copper_nugget', 'netherite_nugget', 'debris_nugget']
WORLD_FEATURES = ['cobalt_ore_small', 'cobalt_ore_large', 'earth_geode', 'sky_geode', 'ichor_geode', 'ender_geode']
# Configured features with no placed feature of their own: the slime trees and fungi are placed by saplings (the growth
# unit) and by the island structures' tree markers, and the two ender trees are vanilla `minecraft:tree` features whose
# Tinkers components (the leave-vines decorator and the extra-root-variants root placer) live in api/src/worldgen.rs.
WORLD_CONFIGURED_FEATURES = ['earth_slime_tree', 'earth_slime_island_tree', 'sky_slime_tree', 'sky_slime_island_tree',
                             'ender_slime_tree', 'ender_slime_tree_tall', 'blood_slime_fungus', 'blood_slime_island_fungus', 'ichor_slime_fungus']
WORLD_STRUCTURES = ['earth_slime_island', 'sky_slime_island', 'ocean_skyslime_island', 'clay_island', 'blood_island', 'end_slime_island']
WORLD_STRUCTURE_SETS = ['overworld_ocean_island', 'overworld_sky_island', 'nether_ocean_island', 'end_sky_island']
WORLD_ISLAND_TAGS = ['blood', 'clay', 'earthslime', 'enderslime', 'skyslime']
# The island template NBTs: the repalleter's output (world/data/StructureRepalleter.java) for the four slime materials
# is generated data; the hand-authored dirt templates it repalettes from are main resources (the clay island places
# the dirt templates unchanged).
WORLD_ISLAND_TEMPLATES = {'blood': 'generated', 'earth': 'generated', 'ender': 'generated', 'sky': 'generated', 'dirt': 'main'}
ISLAND_SIZES = ['0x1x0', '2x2x4', '4x1x6', '8x1x11', '11x1x11']
# The sky and ender geode modifiers select biomes with Forge biome predicates (forge:and/or/not), which the neoforge
# HolderSet form cannot express. The ender predicate (the end, minus the main island) expands exactly to these four
# biomes; the sky predicate (the overworld, minus its oceans, deep oceans, beaches and rivers) expands to the
# explicit set write_biome_spawn_tags computes from the pinned base game's biome tags, shipped as a tag of our own.
WORLD_BIOME_PREDICATES = {
    'sky_geode': '#tconstruct:has_sky_geode',
    'ender_geode': ['minecraft:end_highlands', 'minecraft:end_midlands', 'minecraft:small_end_islands', 'minecraft:end_barrens'],
}
# Items that hold fluid, with their stack size (TinkerFluids: the bottles stack to 16 and the soup bowl to 1, TinkerSmeltery: the copper can to 16).
# The bottles' and the bowl's models are plain; the copper can's own loader is replaced by write_copper_can below.
FLUID_ITEMS = {'venom_bottle': 16, 'earth_slime_bottle': 16, 'sky_slime_bottle': 16, 'ender_slime_bottle': 16, 'ichor_slime_bottle': 16, 'magma_bottle': 16,
               'meat_soup': 1, 'copper_can': 16}
# TinkerSmeltery's casts: registerCast(name) makes <name>_cast (gold), <name>_sand_cast and <name>_red_sand_cast, all plain items
# (PartCastItem only adds the part's tooltip); the arrow cast and the two blank casts are single plain items.
CAST_NAMES = ['ingot', 'nugget', 'gem', 'rod', 'repair_kit', 'plate', 'gear', 'coin', 'wire', 'pick_head', 'small_axe_head', 'small_blade', 'adze_head',
              'hammer_head', 'broad_axe_head', 'broad_blade', 'large_plate', 'tool_binding', 'tough_binding', 'tool_handle', 'tough_handle',
              'bow_limb', 'bow_grip', 'helmet_plating', 'chestplate_plating', 'leggings_plating', 'boots_plating', 'maille']
CAST_ITEMS = [f'{name}{kind}_cast' for name in CAST_NAMES for kind in ('', '_sand', '_red_sand')] + ['arrow_cast', 'blank_sand_cast', 'blank_red_sand_cast']
WORLD_BIOME_MODIFIERS = ['cobalt_ore', 'earth_geode', 'sky_geode', 'ichor_geode', 'ender_geode']


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
        blocks[f'{slime}_congealed_slime']['collision_shapes'] = {'default': [[1, 0, 1, 15, 15, 15]]}
        if slime != 'earth':  # the earth slime block is vanilla's
            # slimeProps: builder(color, SLIME_BLOCK).friction(0.8F).noOcclusion() (no strength: 0); ichor adds its light
            blocks[f'{slime}_slime'] = block('minecraft:slime_block', color, 'minecraft:slime_block', 0.0, 0.0, False, light, friction=0.8, occludes=False)
    for dirt, color in DIRT_TYPES.items():
        # builder(color, SLIME_BLOCK).strength(1.9f); harvest tiers are tags and need no tool (no requiresCorrectToolForDrops)
        blocks[f'{dirt}_slime_dirt'] = block('minecraft:dirt', color, 'minecraft:slime_block', 1.9, 1.9, False)
    blocks.update(world_w1_blocks())
    return blocks


W1_WOODS = {'greenheart': ('COLOR_LIGHT_GREEN', 'COLOR_GREEN'), 'skyroot': ('COLOR_CYAN', 'TERRACOTTA_CYAN'),
            'bloodshroom': ('COLOR_RED', 'COLOR_ORANGE'), 'enderbark': ('COLOR_BLACK', 'COLOR_BLACK')}  # wood -> (planks, bark) MapColor of createSlimewood
W1_FOLIAGE = ['earth', 'sky', 'ichor', 'ender', 'blood']  # FoliageType.values()
W1_FOLIAGE_COLORS = {'earth': 'GRASS', 'sky': 'DIAMOND', 'ichor': 'COLOR_ORANGE', 'ender': 'COLOR_PURPLE', 'blood': 'COLOR_RED'}  # FoliageType.getMapColor
W1_NETHER_FOLIAGE = ('ichor', 'blood')  # FoliageType.isNether: nylium grass, fungus saplings, wart-block leaves
W1_DIRTS = ['vanilla', 'earth', 'sky', 'ichor', 'ender']  # the slime grass EnumObjects, in registration order


def compostable(chance):
    """The `minecraft:compostable` item component for one of TinkerWorld.commonSetup's chances, in the
    shape of vanilla's own compostable providers: an empty composter always takes a layer, a filled one
    takes it with the chance. The host reads the component off the item (a block declaration's components
    patch its block item too) and evaluates the layers provider at each insertion."""
    weight = round(chance * 100)
    return {'minecraft:compostable': {'layers': {
        'type': 'minecraft:number_dispatcher',
        'cases': [{'condition': {'type': 'minecraft:match_block', 'blocks': 'minecraft:composter', 'state': {'level': '0'}}, 'value': 1}],
        'default': {'type': 'minecraft:weighted_list', 'distribution': [{'data': 1, 'weight': weight}, {'data': 0, 'weight': 100 - weight}]}}}}


def world_w1_blocks():
    """name -> declaration of the slime wood sets, the slime grass, the slime flora, the slime vines and the enderbark roots of
    TinkerWorld (the world tab's trees and foliage), each from its Java registration. Templates carry the vanilla behaviour of the
    Java class: the wood pieces are the oak wood blocks (Mantle's WoodBlockObject builds the vanilla block classes, and
    createSlimewood sets the BASS instrument the oak templates already have), the overworld grass is the grass block (SnowyDirtBlock),
    the nylium and the nether leaves are plain blocks (the dirt template: the default instrument of Properties.of()), the leaves are
    the oak leaves, the propagule is the mangrove propagule, the roots are the mangrove roots.

    The wood property function (createSlimewood) sets no strength on any variant, so every wood block is 0/0 like the sticky slime
    blocks above whose property function likewise sets none: the default variant takes the planks map colour and the SLIME_BLOCK
    sound with no tool requirement, the log and wood variants take the bark colour, the WOOD sound and requiresCorrectToolForDrops.
    A log's map colour in the original depends on its axis (the planks colour on the end faces); a declaration carries one colour,
    so the bark colour is declared.

    Behaviour the declarations cannot carry, and where it lives instead: the grass spreading and bonemeal growth
    (SlimeGrassBlock/SlimeNyliumBlock), the saplings growing into trees and huge fungi, the propagule leaves placing a hanging
    propagule when bonemealed and the vines growing through their stages are registered in the api crate's world_growth module;
    the declarations here only carry their random_ticks flag, which the original's registrations set with randomTicks() on the
    grass, the saplings, the propagule and the vines (the fungi get none, as in the original). SlimeLeavesBlock measures its decay distance against the tconstruct:slimy_logs tag rather than logs alone;
    the leaves template's distance is the host's, so the api crate's world_growth module recomputes the slime distance in
    its placement, updateShape and tick callbacks. The signs
    are declared as plain blocks: the original's sign block entities (editable text) have no content declaration, so the signs
    drop and render as in the original but hold no text. Push reactions (the flora and the vines are DESTROY) and the flora's
    render offsets are registrations the declaration schema has no keys for; they are documented here rather than faked.
    The compost values of TinkerWorld.commonSetup ride the declarations' components (the host's compostable item
    component), and the fire spread/burn values are fire odds on the api crate's block behaviours (world_blocks and
    world_growth). The potted blocks and the wall signs have no item in the
    original (the wall signs drop through the standing sign's loot table, which copy_world_loot reproduces)."""
    def declare(template, color, sounds, hardness, resistance, requires_tool, item=True, light=0, components=None, **extra):
        properties = {'hardness': hardness, 'explosion_resistance': resistance, 'sounds': sounds, 'map_color': MAP_COLOR[color],
                      'requires_tool': requires_tool, **extra}
        if light:
            properties['light'] = light
        declaration = {'template': template, 'properties': properties, 'item': item}
        if components:
            declaration['components'] = components
        return declaration

    blocks = {}
    # createSlimewood: (suffix, template, variant); the stripped pieces are prefixed in the block name, not suffixed
    wood_variants = [('planks', 'minecraft:oak_planks', 'default'), ('planks_slab', 'minecraft:oak_slab', 'default'),
                     ('planks_stairs', 'minecraft:oak_stairs', 'default'), ('fence', 'minecraft:oak_fence', 'default'),
                     ('fence_gate', 'minecraft:oak_fence_gate', 'default'), ('door', 'minecraft:oak_door', 'default'),
                     ('trapdoor', 'minecraft:oak_trapdoor', 'default'), ('pressure_plate', 'minecraft:oak_pressure_plate', 'default'),
                     ('button', 'minecraft:oak_button', 'default'), ('sign', 'minecraft:oak_sign', 'default'),
                     ('wall_sign', 'minecraft:oak_wall_sign', 'default'), ('hanging_sign', 'minecraft:oak_hanging_sign', 'default'),
                     ('wall_hanging_sign', 'minecraft:oak_wall_hanging_sign', 'default'),
                     ('log', 'minecraft:oak_log', 'log'), ('stripped_log', 'minecraft:oak_log', 'log'),
                     ('wood', 'minecraft:oak_log', 'wood'), ('stripped_wood', 'minecraft:oak_log', 'wood')]
    for wood, (planks_color, bark_color) in W1_WOODS.items():
        for suffix, template, variant in wood_variants:
            name = f'stripped_{wood}_{suffix[9:]}' if suffix.startswith('stripped_') else f'{wood}_{suffix}'
            if variant == 'default':  # mapColor(planks), SoundType.SLIME_BLOCK
                blocks[name] = declare(template, planks_color, 'minecraft:slime_block', 0.0, 0.0, False, item=suffix not in ('wall_sign', 'wall_hanging_sign'))
            else:  # mapColor(bark), SoundType.WOOD, requiresCorrectToolForDrops
                blocks[name] = declare(template, bark_color, 'minecraft:oak_log', 0.0, 0.0, True)
    for foliage in W1_FOLIAGE:
        color = W1_FOLIAGE_COLORS[foliage]
        nether = foliage in W1_NETHER_FOLIAGE
        for dirt in W1_DIRTS:
            # slimeGrassProps: builder(foliage colour, SLIME_BLOCK).strength(2.0).requiresCorrectToolForDrops().randomTicks()
            if nether:  # SlimeNyliumBlock: a plain block
                blocks[f'{foliage}_{dirt}_slime_grass'] = declare('minecraft:dirt', color, 'minecraft:slime_block', 2.0, 2.0, True, random_ticks=True)
            else:  # SlimeGrassBlock extends SnowyDirtBlock
                blocks[f'{foliage}_{dirt}_slime_grass'] = declare('minecraft:grass_block', color, 'minecraft:slime_block', 2.0, 2.0, True, random_ticks=True)
        # saplingProps: builder(foliage colour, FUNGUS in the nether else GRASS).instabreak().noCollission()
        if foliage == 'ender':  # SlimePropaguleBlock: the mangrove propagule states come with the template
            blocks[f'{foliage}_slime_sapling'] = declare('minecraft:mangrove_propagule', color, 'minecraft:short_grass', 0.0, 0.0, False, random_ticks=True, components=compostable(0.35))
        elif nether:  # SlimeFungusBlock
            blocks[f'{foliage}_slime_sapling'] = declare('minecraft:crimson_fungus', color, 'minecraft:crimson_fungus', 0.0, 0.0, False, components=compostable(0.35))
        else:  # SlimeSaplingBlock
            blocks[f'{foliage}_slime_sapling'] = declare('minecraft:oak_sapling', color, 'minecraft:short_grass', 0.0, 0.0, False, random_ticks=True, components=compostable(0.35))
        if nether:  # a plain block: builder(foliage colour, WART_BLOCK).strength(1.5)
            blocks[f'{foliage}_slime_leaves'] = declare('minecraft:dirt', color, 'minecraft:nether_wart_block', 1.5, 1.5, False, components=compostable(0.85))
        else:  # SlimeLeavesBlock (the propagule leaves for ender): builder(colour, GRASS).strength(1.0).noOcclusion()
            blocks[f'{foliage}_slime_leaves'] = declare('minecraft:oak_leaves', color, 'minecraft:short_grass', 1.0, 1.0, False, occludes=False, components=compostable(0.35))
        # fernProps/tallGrassProps: builder(foliage colour, ROOTS in the nether else GRASS).replaceable().instabreak().noCollission()
        ground_sound = 'minecraft:crimson_roots' if nether else 'minecraft:short_grass'
        blocks[f'{foliage}_slime_fern'] = declare('minecraft:short_grass', color, ground_sound, 0.0, 0.0, False, components=compostable(0.65))
        blocks[f'{foliage}_slime_tall_grass'] = declare('minecraft:short_grass', color, ground_sound, 0.0, 0.0, False, components=compostable(0.35))
        # FlowerPotBlock: no properties beyond the pot itself, and no item
        blocks[f'potted_{foliage}_slime_fern'] = declare('minecraft:flower_pot', 'NONE', 'minecraft:flower_pot', 0.0, 0.0, False, item=False)
        blocks[f'potted_{foliage}_slime_sapling'] = declare('minecraft:flower_pot', 'NONE', 'minecraft:flower_pot', 0.0, 0.0, False, item=False)
    for slime in ('sky', 'ender'):
        # SlimeVineBlock: builder(slime colour, GRASS).replaceable().strength(0.75).noCollission().randomTicks(); the growth stage is its own state
        color, _light = SLIME_TYPES[slime]
        blocks[f'{slime}_slime_vine'] = {**declare('minecraft:vine', color, 'minecraft:short_grass', 0.75, 0.75, False, random_ticks=True),
                                         'state_properties': {'stage': {'type': 'enum', 'values': ['start', 'middle', 'end']}},
                                         'components': compostable(0.5)}
    # SlimeRootsBlock: builder(COLOR_BLACK, MANGROVE_ROOTS).instrument(BASS).strength(0.7).noOcclusion()
    blocks['enderbark_roots'] = declare('minecraft:mangrove_roots', 'COLOR_BLACK', 'minecraft:mangrove_roots', 0.7, 0.7, False, occludes=False, components=compostable(0.4))
    for slime, (color, light) in SLIME_TYPES.items():
        # SlimeDirtBlock: builder(slime colour, MUDDY_MANGROVE_ROOTS).strength(0.7) with the slime light
        blocks[f'{slime}_enderbark_roots'] = declare('minecraft:dirt', color, 'minecraft:muddy_mangrove_roots', 0.7, 0.7, False, light=light)
    return blocks


# The W1 blocks whose blockstates carry real variants or multipart parts (the rest are a single variant or the four facings the
# generic client path already handles): the wood pieces with states, the snowy overworld grass, the propagule and the vines.
WORLD_STATE_BLOCKS = (
    [f'{wood}_{part}' for wood in W1_WOODS for part in ('planks_slab', 'planks_stairs', 'fence', 'fence_gate', 'door', 'trapdoor',
                                                        'pressure_plate', 'button', 'hanging_sign', 'log', 'wood')]
    + [f'stripped_{wood}_{part}' for wood in W1_WOODS for part in ('log', 'wood')]
    + [f'{foliage}_{dirt}_slime_grass' for foliage in ('earth', 'sky', 'ender') for dirt in W1_DIRTS]
    + ['ender_slime_sapling', 'sky_slime_vine', 'ender_slime_vine'])


# TinkerWorld's geodes (registerGeode/GeodeItemObject): each slime crystal makes a crystal block, a budding block and
# four CrystalClusterBlock buds; the metal clusters are CrystalClusterBlocks on clusterProps. A bud's light is the
# type's base light plus the bud's extra, capped at 15; the sounds are the vanilla amethyst sets the original's Sounds
# builds (its per-type pitches are not part of a declaration). The budding block declares random ticks; the growth
# itself is a later unit, as is geode worldgen. The heads are TinkerHeadType's SkullBlocks: the standing head carries
# the item, the wall head none (its loot comes from the standing head, below).
GEODE_TYPES = {'earth': ('COLOR_LIGHT_GREEN', 3), 'sky': ('COLOR_BLUE', 0), 'ichor': ('COLOR_ORANGE', 10), 'ender': ('COLOR_PURPLE', 7)}
GEODE_BUDS = [('{name}_cluster', 7, 5, 'minecraft:amethyst_cluster'), ('small_{name}_bud', 3, 1, 'minecraft:small_amethyst_bud'),
              ('medium_{name}_bud', 4, 2, 'minecraft:medium_amethyst_bud'), ('large_{name}_bud', 5, 4, 'minecraft:large_amethyst_bud')]
METAL_CLUSTERS = {'steel_cluster': ('STONE', 5, 'minecraft:iron_block'), 'cobalt_cluster': ('COLOR_BLUE', 8, 'minecraft:netherite_block'),
                  'knightmetal_cluster': ('GRASS', 12, 'minecraft:netherite_block')}
HEAD_TYPES = ['blaze', 'enderman', 'stray', 'husk', 'drowned', 'spider', 'cave_spider', 'piglin_brute', 'zombified_piglin']


def cluster_boxes(height):
    """AmethystClusterBlock's outline for this height at the buds' offset 3, one box per facing."""
    o = 3
    return {'down': [[o, 16 - height, o, 16 - o, 16, 16 - o]], 'up': [[o, 0, o, 16 - o, height, 16 - o]],
            'north': [[o, o, 16 - height, 16 - o, 16 - o, 16]], 'south': [[o, o, 0, 16 - o, 16 - o, height]],
            'west': [[16 - height, o, o, 16, 16 - o, 16 - o]], 'east': [[0, o, o, height, 16 - o, 16 - o]]}


def cluster_declaration(color, sounds, hardness, light, height):
    """A CrystalClusterBlock: directional, non-occluding, needs the correct tool; outline and collision are one shape."""
    shapes = {'property': 'facing', 'values': cluster_boxes(height)}
    properties = {'hardness': hardness, 'explosion_resistance': hardness, 'sounds': sounds, 'map_color': MAP_COLORS[color],
                  'requires_tool': True, 'occludes': False}
    if light:
        properties['light'] = light
    return {'template': 'minecraft:stone', 'properties': properties,
            'state_properties': {'facing': {'type': 'enum', 'values': ['down', 'up', 'north', 'south', 'west', 'east']},
                                 'waterlogged': {'type': 'boolean'}},
            'shapes': shapes, 'collision_shapes': shapes, 'item': True}


def world_geode_blocks():
    blocks = {}
    for kind, (color, base_light) in GEODE_TYPES.items():
        name = f'{kind}_slime_crystal'
        crystal = {'hardness': 1.5, 'explosion_resistance': 1.5, 'sounds': 'minecraft:amethyst_block', 'map_color': MAP_COLORS[color], 'requires_tool': True}
        if base_light:
            crystal['light'] = base_light
        blocks[f'{name}_block'] = {'template': 'minecraft:stone', 'properties': dict(crystal), 'item': True}
        budding = dict(crystal)
        budding['random_ticks'] = True
        blocks[f'budding_{name}'] = {'template': 'minecraft:stone', 'properties': budding, 'item': True}
        for key, height, extra, sounds in GEODE_BUDS:
            blocks[key.format(name=name)] = cluster_declaration(color, sounds, 1.5, min(base_light + extra, 15), height)
    for name, (color, light, sounds) in METAL_CLUSTERS.items():
        blocks[name] = cluster_declaration(color, sounds, 2.5, light, 7)
    # SkullBlock/WallSkullBlock at strength 1.0 with no tool requirement; the skull's box is its outline and collision.
    skull = {'template': 'minecraft:stone',
             'properties': {'hardness': 1.0, 'explosion_resistance': 1.0, 'sounds': 'minecraft:stone', 'map_color': 0, 'occludes': False},
             'shapes': {'default': [[4, 0, 4, 12, 8, 12]]}, 'collision_shapes': {'default': [[4, 0, 4, 12, 8, 12]]}}
    wall_shapes = {'property': 'facing', 'values': {'north': [[4, 4, 8, 12, 12, 16]], 'south': [[4, 4, 0, 12, 12, 8]],
                                                    'east': [[8, 4, 4, 16, 12, 12]], 'west': [[0, 4, 4, 8, 12, 12]]}}
    for head in HEAD_TYPES:
        blocks[f'{head}_head'] = {**skull, 'state_properties': {'rotation': {'type': 'enum', 'values': [str(i) for i in range(16)]},
                                                                'powered': {'type': 'boolean'}}, 'item': True}
        blocks[f'{head}_wall_head'] = {**skull, 'state_properties': {'facing': {'type': 'enum', 'values': ['north', 'south', 'east', 'west']},
                                                                     'powered': {'type': 'boolean'}},
                                       'shapes': wall_shapes, 'collision_shapes': wall_shapes, 'item': False}
    return blocks


def smeltery_blocks():
    """name -> declaration of the melter, the heater, the two heating structure controllers, the alloyer and the plain structure blocks
    (TinkerSmeltery: the `seared` and `scorched` controller properties, light 13 while active; the melter and the alloyer add noOcclusion;
    `searedSolidProps`/`scorchedSolidProps` for the building blocks, which add the `in_structure` state of SearedBlock). The slab,
    stair, wall and fence variants, the pillar blocks, glass, ladders and every block with a block entity of its own are other units."""
    def controller(occludes, color, sounds, hardness, resistance):
        properties = {'hardness': hardness, 'explosion_resistance': resistance, 'sounds': sounds, 'map_color': MAP_COLOR[color], 'requires_tool': True}
        if not occludes:
            properties['occludes'] = False
        return {'template': 'minecraft:stone', 'properties': properties,
                'state_properties': {'active': {'type': 'boolean'}, 'facing': {'type': 'enum', 'values': ['north', 'south', 'east', 'west']}, 'in_structure': {'type': 'boolean'}},
                'state_light': {'property': 'active', 'values': {'false': 0, 'true': 13}}, 'item': True}

    def structure_block(color, sounds, hardness, resistance, light=0):
        properties = {'hardness': hardness, 'explosion_resistance': resistance, 'sounds': sounds, 'map_color': MAP_COLOR[color], 'requires_tool': True}
        if light:
            properties['light'] = light
        return {'template': 'minecraft:stone', 'properties': properties, 'state_properties': {'in_structure': {'type': 'boolean'}}, 'item': True}

    blocks = {
        'seared_melter': controller(False, 'COLOR_GRAY', 'minecraft:iron_block', 8.0, 28.0),
        'seared_heater': controller(True, 'COLOR_GRAY', 'minecraft:iron_block', 8.0, 28.0),
        'smeltery_controller': controller(True, 'COLOR_GRAY', 'minecraft:iron_block', 8.0, 28.0),
        'foundry_controller': controller(True, 'TERRACOTTA_BROWN', 'minecraft:basalt', 9.0, 35.0),
        'scorched_alloyer': controller(False, 'TERRACOTTA_BROWN', 'minecraft:basalt', 9.0, 35.0),
    }
    for name in ['seared_stone', 'seared_cobble', 'seared_paver', 'seared_bricks', 'seared_cracked_bricks', 'seared_fancy_bricks', 'seared_triangle_bricks']:
        blocks[name] = structure_block('COLOR_GRAY', 'minecraft:iron_block', 3.0, 9.0)
    blocks['seared_lamp'] = structure_block('COLOR_GRAY', 'minecraft:iron_block', 3.0, 9.0, 15)
    for name in ['scorched_bricks', 'scorched_road', 'chiseled_scorched_bricks']:
        blocks[name] = structure_block('TERRACOTTA_BROWN', 'minecraft:basalt', 2.5, 8.0)
    blocks['scorched_lamp'] = structure_block('TERRACOTTA_BROWN', 'minecraft:basalt', 2.5, 8.0, 15)
    # the pouring blocks: searedNonSolidProps(METAL) strength 3/9 and scorchedNonSolidProps(BASALT) 2.5/8, both noOcclusion and
    # requiring the correct tool; their block states are the original's (TableBlock facing + has_item, FaucetBlock facing,
    # ChannelBlock down/powered and one of none/in/out per side). Waterlogging is left out (no fluid logging here).
    flow = {'type': 'enum', 'values': ['none', 'in', 'out']}
    states = {'table': {'facing': {'type': 'enum', 'values': ['north', 'south', 'east', 'west']}, 'has_item': {'type': 'boolean'}},
              'faucet': {'facing': {'type': 'enum', 'values': ['down', 'north', 'south', 'east', 'west']}},
              'channel': {'down': {'type': 'boolean'}, 'powered': {'type': 'boolean'}, 'north': flow, 'south': flow, 'west': flow, 'east': flow}}
    states['basin'] = states['table']
    for kind, (color, hardness, resistance, sound) in {'seared': ('COLOR_GRAY', 3.0, 9.0, 'minecraft:iron_block'), 'scorched': ('TERRACOTTA_BROWN', 2.5, 8.0, 'minecraft:basalt')}.items():
        for block, properties in states.items():
            name = f'{kind}_{block}'
            blocks[name] = {'template': 'minecraft:stone', 'properties': {'hardness': hardness, 'explosion_resistance': resistance, 'sounds': sound, 'map_color': MAP_COLOR[color], 'requires_tool': True, 'occludes': False},
                            'state_properties': properties, 'item': True}
    blocks.update(smeltery_unit6_blocks())
    blocks.update(smeltery_u6b_blocks())
    blocks.update(smeltery_u6c_blocks())
    blocks.update(smeltery_u6d_blocks())
    blocks.update(metal_blocks())
    blocks.update(commons_blocks())
    blocks.update(firewood_blocks())
    return blocks


def smeltery_unit6_blocks():
    """The rest of the structure blocks and the smeltery's fluid interface (TinkerSmeltery; every one requires the correct tool and
    uses structureProps, noOcclusion for the non-solid ones): grout, slabs, stairs, walls, fences, scorched pillars, glass, panes, ladders,
    the drains, ducts and chutes (factor 2 seared, 3 scorched) and the four tank types of each kind. Templates are the vanilla block of the
    same class (the host copies its states and shape); the slabs and stairs of the original are smeltery component block entities
    without an `in_structure` state, the glass, pillars, ladders, drains, ducts and chutes carry it. Soul glass, fluid cannons, lanterns,
    gauges, the casting tank and the proxy tank are other units."""
    facing = {'type': 'enum', 'values': ['north', 'south', 'east', 'west']}
    in_structure = {'type': 'boolean'}
    kinds = {'seared': ('COLOR_GRAY', 'minecraft:iron_block', 3.0, 9.0), 'scorched': ('TERRACOTTA_BROWN', 'minecraft:basalt', 2.5, 8.0)}

    def declare(template, color, sounds, hardness, resistance, states=None, occludes=True, light_states=False):
        properties = {'hardness': hardness, 'explosion_resistance': resistance, 'sounds': sounds, 'map_color': MAP_COLOR[color], 'requires_tool': True}
        if not occludes:
            properties['occludes'] = False
        declaration = {'template': template, 'properties': properties}
        if states:
            declaration['state_properties'] = states
        if light_states:
            declaration['state_light'] = {'property': 'light', 'values': {str(level): level for level in range(16)}}
        declaration['item'] = True
        return declaration

    blocks = {}
    # grout: builder(color, sound).strength(3.0f).friction(0.8F), no tool needed
    for name, color, sound in (('grout', 'COLOR_LIGHT_GRAY', 'minecraft:sand'), ('nether_grout', 'COLOR_BROWN', 'minecraft:soul_soil')):
        blocks[name] = {'template': 'minecraft:dirt', 'properties': {'hardness': 3.0, 'explosion_resistance': 3.0, 'sounds': sound, 'map_color': MAP_COLOR[color], 'requires_tool': False, 'friction': 0.8}, 'item': True}
    for kind, (color, sound, hardness, resistance) in kinds.items():
        base = kind == 'seared'
        # registerBuilding: slab and stairs of stone, paver, bricks (and cobble); registerWallBuilding adds a wall; registerFenceBuilding a fence
        families = {'seared': {'stone': 'slab,stairs', 'cobble': 'slab,stairs,wall', 'paver': 'slab,stairs', 'bricks': 'slab,stairs,wall'},
                    'scorched': {'bricks': 'slab,stairs,fence', 'road': 'slab,stairs'}}[kind]
        for family, parts in families.items():
            for part in parts.split(','):
                template = {'slab': 'minecraft:stone_slab', 'stairs': 'minecraft:stone_stairs', 'wall': 'minecraft:cobblestone_wall', 'fence': 'minecraft:nether_brick_fence'}[part]
                blocks[f'{kind}_{family}_{part}'] = declare(template, color, sound, hardness, resistance)
        blocks[f'{kind}_glass'] = declare('minecraft:glass', color, 'minecraft:glass', hardness, resistance, {'in_structure': in_structure}, False)
        blocks[f'{kind}_tinted_glass'] = declare('minecraft:tinted_glass', color, 'minecraft:glass', hardness, resistance, {'in_structure': in_structure}, False)
        blocks[f'{kind}_glass_pane'] = declare('minecraft:glass_pane', color, 'minecraft:glass', hardness, resistance, None, False)
        # the ladders' own sounds are METAL / BASALT
        blocks[f'{kind}_ladder'] = declare('minecraft:ladder', color, sound, hardness, resistance, {'in_structure': in_structure, 'bottom': {'type': 'boolean'}}, False)
        factor = 2 if base else 3
        for part in ('drain', 'duct', 'chute'):
            blocks[f'{kind}_{part}'] = declare('minecraft:stone', color, sound, hardness * factor, resistance * factor, {'in_structure': in_structure, 'facing': facing})
        for tank in ('fuel_tank', 'fuel_gauge', 'ingot_tank', 'ingot_gauge'):
            light = {'type': 'enum', 'values': [str(level) for level in range(16)]}
            blocks[f'{kind}_{tank}'] = declare('minecraft:stone', color, sound, hardness, resistance, {'in_structure': in_structure, 'light': light}, False, True)
    for name in ('scorched_stone', 'polished_scorched_stone'):
        blocks[name] = declare('minecraft:basalt', 'TERRACOTTA_BROWN', 'minecraft:basalt', 2.5, 8.0, {'in_structure': in_structure})
    return blocks


def smeltery_u6b_blocks():
    """The fluid cannons and the gauges (TinkerSmeltery): a cannon is a tank block of its kind's properties (the end
    cannon takes the seared ones), adding the six-way `facing` and the redstone `triggered` to the tank's `in_structure`
    and `light`; a gauge is Mantle's GaugeBlock: map colour none, no collision, strength 0.5, no occlusion, the clicked
    face as its only state. The outline shape is the template's; the gauge's thin panel is its model's."""
    facing = {'type': 'enum', 'values': ['down', 'up', 'north', 'south', 'west', 'east']}
    light = {'type': 'enum', 'values': [str(level) for level in range(16)]}

    def cannon(color, sounds, hardness, resistance):
        return {'template': 'minecraft:stone',
                'properties': {'hardness': hardness, 'explosion_resistance': resistance, 'sounds': sounds, 'map_color': MAP_COLOR[color], 'requires_tool': True, 'occludes': False},
                'state_properties': {'facing': facing, 'triggered': {'type': 'boolean'}, 'in_structure': {'type': 'boolean'}, 'light': light},
                'state_light': {'property': 'light', 'values': {str(level): level for level in range(16)}}, 'item': True}

    def gauge():
        return {'template': 'minecraft:stone',
                'properties': {'hardness': 0.5, 'explosion_resistance': 0.5, 'sounds': 'minecraft:stone', 'map_color': MAP_COLOR['NONE'], 'requires_tool': True, 'occludes': False},
                'state_properties': {'facing': facing}, 'collision_shapes': {'default': []}, 'item': True}

    return {'seared_fluid_cannon': cannon('COLOR_GRAY', 'minecraft:iron_block', 3.0, 9.0),
            'scorched_fluid_cannon': cannon('TERRACOTTA_BROWN', 'minecraft:basalt', 2.5, 8.0),
            'end_fluid_cannon': cannon('COLOR_GRAY', 'minecraft:iron_block', 3.0, 9.0),
            'copper_gauge': gauge(), 'obsidian_gauge': gauge()}
def smeltery_u6c_blocks():
    """Lanterns and soul glass (TinkerSmeltery; the two soul glass blocks of TinkerCommons that the smeltery soul glass
    is crafted and cast from come with them). The lanterns are SearedLanternBlock: the vanilla lantern, its states and
    shape from the template, plus the `light` state of SearedTankBlock (the block entity keeps it at the held fluid's
    light) over searedNonSolidProps(SoundType.LANTERN); they are no SearedBlock, so they carry no `in_structure`.
    Soul glass is the glass of its unit with noCollission and a speed factor (0.1 the smeltery pair, 0.2 the commons
    pair, whose glassBuilder is strength 0.3, no tool): the host's block declarations have no speed factor key, so
    the factors ride in PlutoLib's movement registry (`pluto_lib::movement`) and the `pluto_movement` mixin applies
    them to the host's movement table at load. The missing collision is `collision_shapes` of no boxes, as the fluid blocks declare
    theirs. The commons pane keeps its collision (only its speed factor differs)."""
    light = {'type': 'enum', 'values': [str(level) for level in range(16)]}
    in_structure = {'type': 'boolean'}
    blocks = {}
    for kind, (color, hardness, resistance) in {'seared': ('COLOR_GRAY', 3.0, 9.0), 'scorched': ('TERRACOTTA_BROWN', 2.5, 8.0)}.items():
        blocks[f'{kind}_lantern'] = {'template': 'minecraft:lantern',
                                     'properties': {'hardness': hardness, 'explosion_resistance': resistance, 'sounds': 'minecraft:lantern', 'map_color': MAP_COLOR[color], 'requires_tool': True, 'occludes': False},
                                     'state_properties': {'light': light},
                                     'state_light': {'property': 'light', 'values': {str(level): level for level in range(16)}},
                                     'item': True}
        glass = {'hardness': hardness, 'explosion_resistance': resistance, 'sounds': 'minecraft:glass', 'map_color': MAP_COLOR[color], 'requires_tool': True, 'occludes': False}
        blocks[f'{kind}_soul_glass'] = {'template': 'minecraft:glass', 'properties': glass, 'state_properties': {'in_structure': in_structure}, 'item': True, 'collision_shapes': {'default': []}}
        blocks[f'{kind}_soul_glass_pane'] = {'template': 'minecraft:glass_pane', 'properties': glass, 'item': True, 'collision_shapes': {'default': []}}
    commons = {'hardness': 0.3, 'explosion_resistance': 0.3, 'sounds': 'minecraft:glass', 'map_color': MAP_COLOR['COLOR_BROWN'], 'requires_tool': False, 'occludes': False}
    blocks['soul_glass'] = {'template': 'minecraft:glass', 'properties': commons, 'item': True, 'collision_shapes': {'default': []}}
    blocks['soul_glass_pane'] = {'template': 'minecraft:glass_pane', 'properties': commons, 'item': True}
    return blocks


# Their block states are the original's own (variants or multipart over the template's properties), so they are copied as they are.
SMELTERY_STATE_BLOCKS += [name for name in smeltery_unit6_blocks() if name not in ('grout', 'nether_grout')]
SMELTERY_STATE_BLOCKS += list(smeltery_u6b_blocks())
SMELTERY_STATE_BLOCKS += list(smeltery_u6c_blocks())


def smeltery_u6d_blocks():
    """The casting tank and the proxy tank (TinkerSmeltery): the casting tank takes the seared non-solid properties
    (metal sound, strength 3/9) with a horizontal facing and the tanks' `light` state, the proxy tank the scorched
    ones (basalt, 2.5/8) with the facing only; both take the stone template, no occlusion, and require the correct
    tool."""
    facing = {'type': 'enum', 'values': ['north', 'south', 'east', 'west']}
    light = {'type': 'enum', 'values': [str(level) for level in range(16)]}

    def declare(color, sounds, hardness, resistance, states=None, light_states=False):
        properties = {'hardness': hardness, 'explosion_resistance': resistance, 'sounds': sounds, 'map_color': MAP_COLOR[color], 'requires_tool': True, 'occludes': False}
        declaration = {'template': 'minecraft:stone', 'properties': properties}
        if states:
            declaration['state_properties'] = states
        if light_states:
            declaration['state_light'] = {'property': 'light', 'values': {str(level): level for level in range(16)}}
        declaration['item'] = True
        return declaration

    return {
        # CastingTankBlock: searedNonSolidProps(METAL).lightLevel(LIGHT_GETTER), FACING + LIGHT
        'seared_casting_tank': declare('COLOR_GRAY', 'minecraft:iron_block', 3.0, 9.0, {'facing': facing, 'light': light}, True),
        # ProxyTankBlock: scorchedNonSolidProps(BASALT), HORIZONTAL_FACING
        'scorched_proxy_tank': declare('TERRACOTTA_BROWN', 'minecraft:basalt', 2.5, 8.0, {'facing': facing}),
    }


SMELTERY_STATE_BLOCKS += list(smeltery_u6d_blocks())
# The geode buds and clusters key their blockstates by facing alone, so they take the same pass-through.
SMELTERY_STATE_BLOCKS += [name for name in world_geode_blocks() if name.endswith(('_cluster', '_bud'))]

def metal_blocks():
    """The storage blocks of the shipped metals (TinkerMaterials `registerMetal`): `metalBuilder` (metal sound,
    strength 5, the correct tool) with each metal's own map colour and sound; the ones registered with
    `noOcclusion` say so. The block classes are not carried: SlimesteelBlock (slimesteel, cinderslime,
    knightslime) is a slime block — it sticks to slime blocks, negates fall damage and bounces — riding on the
    same documented host stubs as the world unit's slime blocks; the orientable pig iron block's facing is not
    declared (the north variant is the block's one model, as for the world unit's facing blocks);
    KnightMetalBlock's bite is the entity-inside gap of the fluid cannons. The ingots and nuggets are
    METAL_ITEMS.

    Three of the block models do not come across: slimesteel, cinderslime and knightslime are
    `forge:composite` models (a solid metal core under a translucent slime shell), which the flat
    model copy cannot express, so those blocks stand declared with no model, like the scorched proxy
    tank. The queens slime block and the queens slime ingot and nugget carry luminosity-only Mantle
    loaders and import flattened (see PLAIN_LOADERS)."""
    def metal(color, sounds, occludes=True):
        properties = {'hardness': 5.0, 'explosion_resistance': 5.0, 'sounds': sounds, 'map_color': MAP_COLOR[color], 'requires_tool': True}
        if not occludes:
            properties['occludes'] = False
        return {'template': 'minecraft:stone', 'properties': properties, 'item': True}

    metal_sound, netherite_sound = 'minecraft:iron_block', 'minecraft:netherite_block'
    return {
        'cobalt_block': metal('COLOR_BLUE', metal_sound),
        'steel_block': metal('STONE', metal_sound),
        'slimesteel_block': metal('WARPED_WART_BLOCK', metal_sound, False),
        'amethyst_bronze_block': metal('COLOR_PURPLE', metal_sound),
        'rose_gold_block': metal('TERRACOTTA_WHITE', metal_sound),
        'pig_iron_block': metal('COLOR_PINK', metal_sound),
        'cinderslime_block': metal('COLOR_ORANGE', netherite_sound, False),
        'queens_slime_block': metal('COLOR_GREEN', netherite_sound),
        'manyullyn_block': metal('COLOR_PURPLE', netherite_sound),
        'hepatizon_block': metal('TERRACOTTA_BLUE', netherite_sound),
        'knightmetal_block': metal('GRASS', netherite_sound, False),
        'knightslime_block': metal('COLOR_BLACK', metal_sound, False),
        'soulsteel_block': metal('COLOR_BROWN', metal_sound, False),
    }


# The dye colours of the clear stained glass, in GlassColor (dye) order, each with its dye's map colour.
GLASS_COLORS = [('white', 'SNOW'), ('orange', 'COLOR_ORANGE'), ('magenta', 'COLOR_MAGENTA'), ('light_blue', 'COLOR_LIGHT_BLUE'),
                ('yellow', 'COLOR_YELLOW'), ('lime', 'COLOR_LIGHT_GREEN'), ('pink', 'COLOR_PINK'), ('gray', 'COLOR_GRAY'),
                ('light_gray', 'COLOR_LIGHT_GRAY'), ('cyan', 'COLOR_CYAN'), ('purple', 'COLOR_PURPLE'), ('blue', 'COLOR_BLUE'),
                ('brown', 'COLOR_BROWN'), ('green', 'COLOR_GREEN'), ('red', 'COLOR_RED'), ('black', 'COLOR_BLACK')]


def commons_blocks():
    """TinkerCommons blocks the plain crafting recipes name: the clear glass family, the obsidian pane and the
    gold bars. Clear glass is the original's `glassBuilder` (strength 0.3, glass sound, no occlusion) on the
    vanilla glass and glass pane templates; its connected textures ride on the models' `connection` data, which
    the asset import keeps with the edge sprites (the connected rendering itself is the world unit's). The
    obsidian pane is Mantle's BetterPaneBlock at obsidian strength (25/400, stone sound, the correct tool) and
    the gold bars the vanilla IronBarsBlock (metal sound, strength 3/6, the correct tool). The platforms are the
    PlatformBlocks (copper sound, the correct tool, no occlusion) with their six connection states; the shapes the
    connections make and the waterlogging are not carried, as the pouring blocks leave waterlogging out, and the
    copper ones' weathering and waxing are block class behaviour, which is not carried, as the metal blocks'
    classes are not. The cheese block is the cheese pair's HalfTransparentBlock (honey sound, strength 1.5/3, no
    occlusion); its speed (0.4) and jump (0.5) factors ride in PlutoLib's movement registry with the soul glass's,
    applied by the `pluto_movement` mixin, as the block declarations have no key for them."""
    def glass(template, color):
        return {'template': template, 'properties': {'hardness': 0.3, 'explosion_resistance': 0.3, 'sounds': 'minecraft:glass', 'map_color': MAP_COLOR[color], 'requires_tool': False, 'occludes': False}, 'item': True}

    blocks = {'clear_glass': glass('minecraft:glass', 'NONE'), 'clear_glass_pane': glass('minecraft:glass_pane', 'NONE'),
              'clear_tinted_glass': glass('minecraft:tinted_glass', 'COLOR_GRAY')}
    for name, color in GLASS_COLORS:
        blocks[f'{name}_clear_stained_glass'] = glass('minecraft:glass', color)
        blocks[f'{name}_clear_stained_glass_pane'] = glass('minecraft:glass_pane', color)
    blocks['obsidian_pane'] = {'template': 'minecraft:glass_pane',
                               'properties': {'hardness': 25.0, 'explosion_resistance': 400.0, 'sounds': 'minecraft:stone', 'map_color': MAP_COLOR['COLOR_BLACK'], 'requires_tool': True, 'occludes': False},
                               'item': True}
    blocks['gold_bars'] = {'template': 'minecraft:iron_bars',
                           'properties': {'hardness': 3.0, 'explosion_resistance': 6.0, 'sounds': 'minecraft:iron_block', 'map_color': MAP_COLOR['NONE'], 'requires_tool': True, 'occludes': False},
                           'item': True}
    connections = {side: {'type': 'boolean'} for side in ('up', 'down', 'north', 'east', 'south', 'west')}

    def platform(color, hardness, resistance):
        return {'template': 'minecraft:stone', 'properties': {'hardness': hardness, 'explosion_resistance': resistance, 'sounds': 'minecraft:copper_block', 'map_color': MAP_COLOR[color], 'requires_tool': True, 'occludes': False},
                'state_properties': connections, 'item': True}

    for name, color, hardness, resistance in [('copper_platform', 'COLOR_ORANGE', 3.0, 6.0), ('exposed_copper_platform', 'TERRACOTTA_LIGHT_GRAY', 3.0, 6.0),
                                              ('weathered_copper_platform', 'WARPED_STEM', 3.0, 6.0), ('oxidized_copper_platform', 'WARPED_NYLIUM', 3.0, 6.0),
                                              ('waxed_copper_platform', 'COLOR_ORANGE', 3.0, 6.0), ('waxed_exposed_copper_platform', 'TERRACOTTA_LIGHT_GRAY', 3.0, 6.0),
                                              ('waxed_weathered_copper_platform', 'WARPED_STEM', 3.0, 6.0), ('waxed_oxidized_copper_platform', 'WARPED_NYLIUM', 3.0, 6.0),
                                              ('iron_platform', 'METAL', 5.0, 6.0), ('gold_platform', 'GOLD', 3.0, 6.0), ('cobalt_platform', 'COLOR_BLUE', 5.0, 5.0)]:
        blocks[name] = platform(color, hardness, resistance)
    blocks['cheese_block'] = {'template': 'minecraft:stone',
                              'properties': {'hardness': 1.5, 'explosion_resistance': 3.0, 'sounds': 'minecraft:honey_block', 'map_color': MAP_COLOR['COLOR_YELLOW'], 'requires_tool': False, 'occludes': False},
                              'item': True}
    return blocks


# The commons panes and the gold bars: their block states are multipart, copied as they are.
SMELTERY_STATE_BLOCKS += [name for name in commons_blocks() if name.endswith('_pane') or name == 'gold_bars']
# The platforms' block states are multipart over their connection states: copied as they are.
SMELTERY_STATE_BLOCKS += [name for name in commons_blocks() if name.endswith('_platform')]
# The one commons item that is not a block: the cheese ingot, declared a plain item (CheeseItem's food, nutrition 3
# and saturation 0.4, and the random effect its bite cures are not carried by an item declaration).
COMMONS_ITEMS = ['cheese_ingot']



def firewood_blocks():
    """The two firewood families of TinkerMaterials (the nahuatl and blazewood FenceBuildingBlockObjects): the block,
    its fence, its slab and its stairs, at strength 25/300 with the correct tool and the wood sound; blazewood gives
    light 7, and every block of its family carries the light, as the family shares its properties. Templates are the
    vanilla block of the same class, as the smeltery building blocks take theirs."""
    def declare(template, color, light=0):
        properties = {'hardness': 25.0, 'explosion_resistance': 300.0, 'sounds': 'minecraft:oak_planks', 'map_color': MAP_COLOR[color], 'requires_tool': True}
        if light:
            properties['light'] = light
        return {'template': template, 'properties': properties, 'item': True}

    blocks = {}
    for name, color, light in [('nahuatl', 'COLOR_PURPLE', 0), ('blazewood', 'TERRACOTTA_RED', 7)]:
        blocks[name] = declare('minecraft:stone', color, light)
        blocks[f'{name}_fence'] = declare('minecraft:nether_brick_fence', color, light)
        blocks[f'{name}_slab'] = declare('minecraft:stone_slab', color, light)
        blocks[f'{name}_stairs'] = declare('minecraft:stone_stairs', color, light)
    # The original's data ships a fence gate for nahuatl only; blazewood has none.
    blocks['nahuatl_fence_gate'] = declare('minecraft:oak_fence_gate', 'COLOR_PURPLE')
    return blocks


# The firewood fences' block states are multipart and the slabs' and stairs' the original's variants: copied as they are.
SMELTERY_STATE_BLOCKS += [f'{wood}_{part}' for wood in ('nahuatl', 'blazewood') for part in ('fence', 'slab', 'stairs')]


def write_vanilla_fuels():
    """Vanilla's burn times (ForgeHooks.getBurnTime falls back to the furnace fuel map for the melter's heater): the
    `minecraft:cooking_fuel` item component of the pinned base export, its burn time provider evaluated for an ordinary
    (not fast-cooking) block. The host exposes no burn time to mods, so the table is data of the pinned game version."""
    base = ROOT / '.local/base-26.3'
    providers = base / 'data/minecraft/context_int_provider'

    def evaluate(reference):
        name = reference.split(':', 1)[1]
        node = json.loads((providers / f'{name}.json').read_text(encoding='utf-8'))
        return value(node)

    def value(node):
        if isinstance(node, (int, float)):
            return node
        if isinstance(node, str):
            return evaluate(node)
        kind = node['type']
        if kind == 'minecraft:div':
            return int(value(node['left']) / value(node['right']))
        if kind == 'minecraft:conditional':
            return value(node['on_false'])  # block/fast_cooking is the only condition and does not hold
        if kind == 'minecraft:constant':
            return value(node['value'])
        raise SystemExit(f'burn time provider {kind} is not known')

    items = json.loads((base / 'items.json').read_text(encoding='utf-8'))
    fuels = {name: value(item['minecraft:cooking_fuel']['burn_time']) for name, item in sorted(items.items()) if 'minecraft:cooking_fuel' in item}
    lines = ['//! Vanilla furnace fuels of the pinned game version: item and burn time in ticks (the `minecraft:cooking_fuel` item',
             '//! component, evaluated for an ordinary block). Generated from the pinned base export by tools/import_original_data.py.',
             '', 'pub(crate) const FUELS: &[(&str, i32)] = &['] + [f'    ("{name}", {ticks}),' for name, ticks in fuels.items()] + ['];']
    (ROOT / 'tconstruct/mixins/tconstruct/api/src/vanilla_fuels.rs').write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
    return len(fuels)


def gadget_blocks():
    """name -> declaration of the gadgets whose behaviour needs no host service this port lacks (TinkerGadgets): the punji
    sticks and the slime cakes. Punji: builder(MapColor.PLANT, SoundType.GRASS).strength(3.0F).noOcclusion(); its facing and
    the four connection states are its own (waterlogging is left out, as with the pouring blocks), and the one collision box
    is the floor shape, as the declaration holds a single default (the host reads per-facing shapes from no key here).
    The cakes ride the vanilla cake block, whose template carries the bites state and the floor cakes' survival and shapes;
    theirs is builder(SoundType.WOOL).strength(0.5F) with map colour NONE, and the hanging ichor cake overrides survival in
    the api crate. The ichor cake's one collision box is its unbitten shape (bites narrow it in the original)."""
    blocks = {
        'punji': {'template': 'minecraft:stone',
                  'properties': {'hardness': 3.0, 'explosion_resistance': 3.0, 'sounds': 'minecraft:grass_block', 'map_color': MAP_COLOR['PLANT'], 'requires_tool': False, 'occludes': False},
                  'state_properties': {'facing': {'type': 'enum', 'values': ['down', 'up', 'north', 'south', 'west', 'east']},
                                       'north': {'type': 'boolean'}, 'east': {'type': 'boolean'}, 'northeast': {'type': 'boolean'}, 'northwest': {'type': 'boolean'}},
                  'default_state': {'facing': 'down', 'north': 'false', 'east': 'false', 'northeast': 'false', 'northwest': 'false'},
                  'collision_shapes': {'default': [[3, 0, 3, 13, 6, 13]]}, 'item': True},
    }
    cake = {'template': 'minecraft:cake',
            'properties': {'hardness': 0.5, 'explosion_resistance': 0.5, 'sounds': 'minecraft:white_wool', 'map_color': MAP_COLOR['NONE'], 'requires_tool': False},
            'item': True}
    for name in ('earth_cake', 'sky_cake', 'ender_cake', 'blood_cake', 'magma_cake'):
        blocks[name] = cake
    blocks['ichor_cake'] = {**cake, 'collision_shapes': {'default': [[1, 8, 1, 15, 16, 15]]}}
    return blocks


SMELTERY_STATE_BLOCKS += list(gadget_blocks())


def write_world_declarations():
    """Block and item declarations of the world units, written beside the hand-written station ones."""
    blocks = {**world_blocks(), **world_geode_blocks(), **smeltery_blocks(), **gadget_blocks()}
    for name, declaration in blocks.items():
        write_json(HORIZON_DECLARATIONS / 'block' / f'{name}.json', declaration, compact=True)
    for name in WORLD_ITEMS + CAST_ITEMS + METAL_ITEMS + COMMONS_ITEMS:
        # The slime grass seeds compost at TinkerWorld's 0.35, like the foliage they grow into.
        declaration = {'template': 'minecraft:paper', 'components': compostable(0.35)} if name.endswith('_slime_grass_seeds') else {'template': 'minecraft:paper'}
        write_json(HORIZON_DECLARATIONS / 'item' / f'{name}.json', declaration, compact=True)
    for name, stack in FLUID_ITEMS.items():
        write_json(HORIZON_DECLARATIONS / 'item' / f'{name}.json', {'template': 'minecraft:paper', 'max_stack_size': stack}, compact=True)
    return len(blocks), len(WORLD_ITEMS) + len(CAST_ITEMS) + len(METAL_ITEMS) + len(COMMONS_ITEMS) + len(FLUID_ITEMS)


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
            # The heating controllers' tables also copy the block entity's name (minecraft:copy_name): no surface
            # here hands a drop the block entity's name, so that function is dropped and the plain drop stays.
            # The retexture (mantle:fill_retextured_block) is kept: the port's own loot function applies it.
            kept = [upgrade_function(function) for function in value if function['function'] != 'minecraft:copy_name']
            if kept:
                result['modifier'] = kept
        elif key == 'bonus_rolls' and value == 0:
            continue  # the default
        elif key in ('type', 'name', 'random_sequence', 'rolls'):
            result[key] = value
        elif key in ('pools', 'entries', 'children'):
            result[key] = upgrade_loot(value)
        else:
            raise SystemExit(f'loot table: unknown key {key!r}')
    if 'random_sequence' in result and 'pools' not in result:
        # A block that drops nothing (the budding crystals) gets no pools from datagen; the port's
        # loot schema spells the empty list out.
        result['pools'] = []
    return result


def upgrade_condition(condition):
    kind = condition['condition']
    if kind == 'minecraft:survives_explosion' and set(condition) == {'condition'}:
        return {'type': kind}
    silk = {'enchantments': [{'enchantment': 'minecraft:silk_touch', 'levels': {'min': 1}}]}
    if kind == 'minecraft:match_tool' and condition['predicate'] == silk:
        return 'minecraft:tool/can_silk_touch'
    if kind == 'minecraft:match_tool' and condition.get('predicate') == {'items': ['minecraft:shears']}:
        return 'minecraft:tool/can_shear'
    if kind == 'forge:can_tool_perform_action' and set(condition) == {'condition', 'action'} and condition['action'] == 'shears_dig':
        # Forge's tool action is the host's tool condition string, as the base game's own fern and vine tables write it
        return 'minecraft:tool/can_shear'
    if kind == 'minecraft:table_bonus' and set(condition) == {'condition', 'enchantment', 'chances'}:
        return {'type': kind, 'enchantment': condition['enchantment'], 'chances': condition['chances']}
    if kind == 'minecraft:any_of' and set(condition) == {'condition', 'terms'}:
        return {'type': kind, 'terms': [upgrade_condition(term) for term in condition['terms']]}
    if kind == 'minecraft:inverted' and set(condition) == {'condition', 'term'}:
        return {'type': kind, 'term': upgrade_condition(condition['term'])}
    if kind == 'minecraft:block_state_property' and set(condition) == {'condition', 'block', 'properties'}:
        return {'type': kind, 'block': condition['block'], 'properties': condition['properties']}
    if kind == 'minecraft:match_tool' and set(condition) == {'condition', 'predicate'} and set(condition['predicate']) == {'tag'}:
        # 26.3 names the tool tag in the predicate's `items`, as its own amethyst cluster table does.
        return {'type': kind, 'predicate': {'items': f"#{condition['predicate']['tag']}"}}
    if kind == 'minecraft:damage_source_properties' and set(condition) == {'condition', 'predicate'}:
        # The entity loot tables: the killer predicate (a frog, in the original's slime tables) carries over as it stands.
        return {'type': kind, 'predicate': condition['predicate']}
    if kind == 'minecraft:entity_properties' and set(condition) == {'condition', 'entity', 'predicate'}:
        # The entity loot tables: the slime size, the metal NBT flag and the on-fire flag predicates carry over as they stand.
        return {'type': kind, 'entity': condition['entity'], 'predicate': condition['predicate']}
    raise SystemExit(f'loot table: unknown condition {condition}')


def upgrade_function(function):
    kind = function['function']
    rest = {key: value for key, value in function.items() if key != 'function'}
    if kind == 'minecraft:explosion_decay' and not rest:
        return {'type': kind}
    if kind == 'minecraft:apply_bonus' and rest == {'enchantment': 'minecraft:fortune', 'formula': 'minecraft:ore_drops'}:
        return {'type': kind, **rest}
    if kind == 'minecraft:set_count' and rest.get('add') is False and set(rest) <= {'add', 'count', 'conditions'}:
        result = {'type': kind, 'count': rest['count']}
        if 'conditions' in rest:
            terms = [upgrade_condition(term) for term in rest['conditions']]
            result['condition'] = terms[0] if len(terms) == 1 else {'type': 'minecraft:all_of', 'terms': terms}
        return result
    if kind == 'minecraft:looting_enchant' and set(rest) == {'count'}:
        return {'type': kind, 'count': rest['count']}
    if kind == 'minecraft:furnace_smelt' and set(rest) <= {'conditions'}:
        # The terracube table smelts its whole pool for a burning cube (the on-fire entity condition).
        result = {'type': kind}
        if 'conditions' in rest:
            terms = [upgrade_condition(term) for term in rest['conditions']]
            result['condition'] = terms[0] if len(terms) == 1 else {'type': 'minecraft:all_of', 'terms': terms}
        return result
    if kind == 'minecraft:copy_nbt' and rest.get('source') == 'block_entity' and set(rest) == {'source', 'ops'}:
        # the tank items keep their fluid in the stack's custom data under `tank` (TankItem.getTank reads the stack's `tank` tag)
        return {'type': 'minecraft:copy_custom_data', 'source': 'block_entity', 'ops': rest['ops']}
    if kind == 'mantle:fill_retextured_block' and not rest:
        # A kind of the port's own: the loot engine hands a kind outside `minecraft:` to the mod's loot
        # functions (PlutoLib's LootTableEvents::FUNCTION), which read the block entity from the roll's place.
        return {'type': kind}
    raise SystemExit(f'loot table: unknown function {function}')


def copy_world_loot(names):
    """Block loot tables (data/<ns>/loot_table/blocks, 26.3's folder) of the given blocks."""
    target = CONTENT_DATA / 'tconstruct/loot_table'
    shutil.rmtree(target, ignore_errors=True)
    for name in names:
        source = GENERATED_DATA / f'tconstruct/loot_tables/blocks/{name}.json'
        if not source.exists() and name.endswith('_wall_head'):
            # WallSkullBlock.lootFrom the standing head: datagen writes no table of its own, so the head's stands in.
            source = GENERATED_DATA / f"tconstruct/loot_tables/blocks/{name.removesuffix('_wall_head')}_head.json"
        if not source.exists():
            # the wall signs have no table of their own: Mantle's wood object points them at the standing sign's table
            source = GENERATED_DATA / f'tconstruct/loot_tables/blocks/{name.replace("_wall_", "_")}.json'
        if not source.exists() and name.endswith('_wall_head'):
            # WallSkullBlock.lootFrom the standing head: datagen writes no table of its own, so the head's stands in.
            source = GENERATED_DATA / f"tconstruct/loot_tables/blocks/{name.removesuffix('_wall_head')}_head.json"
        table = whole(upgrade_loot(json.loads(source.read_text(encoding='utf-8'))))
        if 'pools' not in table:
            table['pools'] = []  # the cakes' tables name no pool: eaten away, they drop nothing, and the shape here requires the list
        write_json(target / 'blocks' / f'{name}.json', table)
    return len(names)


def upgrade_feature(feature):
    """A 1.20.1 configured feature in 26.3's `worldgen/feature` form: the config is the feature's own body and a plain
    block state anywhere inside it (an ore target, a state provider's state, a fungus state) is its name; a state with
    properties stays an object."""
    body = {'type': feature['type'], **feature['config']}

    def convert(node):
        if isinstance(node, dict):
            if 'Name' in node and set(node) <= {'Name', 'Properties'} and 'Properties' not in node:
                return node['Name']
            return {key: convert(value) for key, value in node.items()}
        if isinstance(node, list):
            return [convert(value) for value in node]
        return node

    return convert(body)


def write_biome_spawn_tags():
    """The explicit biome sets the original's Forge biome predicates name, expanded from the pinned base
    game's biome tags: sky geodes generate in the overworld minus its oceans, deep oceans, beaches and
    rivers, and terracubes spawn in the overworld minus the forge:no_default_monsters biomes (the mushroom
    fields and the deep dark). The sets ship as tconstruct biome tags, which the modifiers name."""
    base = ROOT / '.local/base-26.3/data/minecraft/tags/worldgen/biome'

    def flatten(name):
        values = []
        for entry in json.loads((base / f'{name}.json').read_text(encoding='utf-8'))['values']:
            entry = entry if isinstance(entry, str) else entry['id']
            if entry.startswith('#'):
                values += flatten(entry.split(':', 1)[1])
            else:
                values.append(entry)
        return values

    overworld = set(flatten('is_overworld'))
    water = set()
    for tag in ('is_ocean', 'is_deep_ocean', 'is_beach', 'is_river'):
        water |= set(flatten(tag))
    write_json(CONTENT_DATA / 'tconstruct/tags/worldgen/biome/has_sky_geode.json', {'values': sorted(overworld - water)})
    no_default = json.loads((GENERATED_DATA / 'forge/tags/worldgen/biome/no_default_monsters.json').read_text(encoding='utf-8'))['values']
    excluded = {entry if isinstance(entry, str) else entry['id'] for entry in no_default}
    write_json(CONTENT_DATA / 'tconstruct/tags/worldgen/biome/terracube_spawns.json', {'values': sorted(overworld - excluded)})
    return 2


def copy_world_features():
    """Cobalt's configured and placed features and its biome modifier (the host reads `neoforge/biome_modifier` and the
    `forge:`/`neoforge:` types alike; the original's `forge:add_features` becomes `neoforge:add_features`)."""
    for folder in ('worldgen', 'neoforge'):
        shutil.rmtree(CONTENT_DATA / 'tconstruct' / folder, ignore_errors=True)
    source = GENERATED_DATA / 'tconstruct'
    for name in WORLD_FEATURES:
        feature = upgrade_feature(json.loads((source / f'worldgen/configured_feature/{name}.json').read_text(encoding='utf-8')))
        for target in feature.get('targets', []):
            block = target['state'] if isinstance(target['state'], str) else target['state']['Name']
            if block.startswith('tconstruct:') and not (HORIZON_DECLARATIONS / 'block' / f'{block[len("tconstruct:"):]}.json').exists():
                raise SystemExit(f'feature {name} places the undeclared block {block}')
        write_json(CONTENT_DATA / f'tconstruct/worldgen/feature/{name}.json', feature)
        placed = json.loads((source / f'worldgen/placed_feature/{name}.json').read_text(encoding='utf-8'))
        if placed['feature'] != f'tconstruct:{name}':
            raise SystemExit(f'placed feature {name} names {placed["feature"]}')
        write_json(CONTENT_DATA / f'tconstruct/worldgen/placed_feature/{name}.json', placed)
    for name in WORLD_CONFIGURED_FEATURES:
        feature = upgrade_feature(json.loads((source / f'worldgen/configured_feature/{name}.json').read_text(encoding='utf-8')))
        write_json(CONTENT_DATA / f'tconstruct/worldgen/feature/{name}.json', feature)
    for name in WORLD_BIOME_MODIFIERS:
        modifier = json.loads((source / f'forge/biome_modifier/{name}.json').read_text(encoding='utf-8'))
        features = [modifier['features']] if isinstance(modifier['features'], str) else modifier['features']
        if not modifier['type'].startswith('forge:') or not set(features) <= {f'tconstruct:{n}' for n in WORLD_FEATURES + WORLD_CONFIGURED_FEATURES}:
            raise SystemExit(f'biome modifier {name} is not a forge modifier of the shipped features')
        modifier['type'] = 'neoforge:' + modifier['type'].removeprefix('forge:')
        if isinstance(modifier['biomes'], dict):
            modifier['biomes'] = WORLD_BIOME_PREDICATES[name]
        write_json(CONTENT_DATA / f'tconstruct/neoforge/biome_modifier/{name}.json', modifier)
    return len(WORLD_FEATURES) * 2 + len(WORLD_CONFIGURED_FEATURES) + len(WORLD_BIOME_MODIFIERS) + copy_world_structures() + write_biome_spawn_tags()


def copy_world_structures():
    """The slime islands: the six `tconstruct:island` structures and their four random-spread sets ship as the
    original's data (the consumer codecs are api/src/worldgen.rs), with the five island biome tags and the 25 island
    template NBTs (each twice: the original's `structures/` folder and the host template loader's `structure/`
    folder, as the copy loop explains). The structures' spawn overrides name the slime entity types of the later
    slime-mob unit; the data ships as the original has it."""
    source = GENERATED_DATA / 'tconstruct'
    count = 0
    for name in WORLD_STRUCTURES:
        structure = json.loads((source / f'worldgen/structure/{name}.json').read_text(encoding='utf-8'))
        write_json(CONTENT_DATA / f'tconstruct/worldgen/structure/{name}.json', structure)
        count += 1
    for name in WORLD_STRUCTURE_SETS:
        structure_set = json.loads((source / f'worldgen/structure_set/{name}.json').read_text(encoding='utf-8'))
        write_json(CONTENT_DATA / f'tconstruct/worldgen/structure_set/{name}.json', structure_set)
        count += 1
    for name in WORLD_ISLAND_TAGS:
        tag = json.loads((source / f'tags/worldgen/biome/islands/{name}.json').read_text(encoding='utf-8'))
        write_json(CONTENT_DATA / f'tconstruct/tags/worldgen/biome/islands/{name}.json', tag)
        count += 1
    for material, origin in WORLD_ISLAND_TEMPLATES.items():
        base = (GENERATED_DATA if origin == 'generated' else MAIN_DATA) / 'tconstruct/structures/islands' / material
        for size in ISLAND_SIZES:
            target = CONTENT_DATA / f'tconstruct/structures/islands/{material}/{size}.nbt'
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(base / f'{size}.nbt', target)
            count += 1
            # The host's content-mod builder forwards only `data/<ns>/structure/**.nbt` (singular) into the
            # worldgen loader's template folder; the original's plural folder is never read there, so each
            # template ships a second, identical copy under the singular folder — without it the loader's
            # template store stays empty and every island start is rejected as a missing template.
            host_target = CONTENT_DATA / f'tconstruct/structure/islands/{material}/{size}.nbt'
            host_target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(base / f'{size}.nbt', host_target)
            count += 1
    return count


SLIME_ENTITIES = ['sky_slime', 'ender_slime', 'terracube']
SLIME_SPAWN_MODIFIERS = ['spawn_overworld_slime', 'spawn_terracube', 'spawn_end_slime']


def write_slime_entities():
    """The slime mobs' data: a spawn egg declaration per type (the vanilla slime egg carrying the type in its
    `minecraft:entity_data`, the form the base game's own eggs take), the entity loot tables through the same
    upgrade as the block tables, and the natural-spawn biome modifiers (`forge:add_spawns` becomes
    `neoforge:add_spawns`). Runs after copy_world_loot and copy_world_features, whose directory wipes it must
    follow. The entity types themselves, their class behaviour and their (unregistered) spawn
    placements live in the api module `slime_entities.rs`. The terracube predicate's exclusion (the
    forge:no_default_monsters biomes) ships expanded as the terracube_spawns biome tag
    (write_biome_spawn_tags), so the modifier names that tag instead of widening to the overworld."""
    for name in SLIME_ENTITIES:
        write_json(HORIZON_DECLARATIONS / 'item' / f'{name}_spawn_egg.json',
                   {'template': 'minecraft:slime_spawn_egg', 'components': {'minecraft:entity_data': {'id': f'tconstruct:{name}'}}}, compact=True)
        table = json.loads((GENERATED_DATA / 'tconstruct/loot_tables/entities' / f'{name}.json').read_text(encoding='utf-8'))
        write_json(CONTENT_DATA / 'tconstruct/loot_table/entities' / f'{name}.json', whole(upgrade_loot(table)))
    # The fancy gadgets' armor stand entity is registered with the gadgets; the original's loot table for
    # it is empty (the stand returns its item through its own death logic, not through the table).
    table = json.loads((GENERATED_DATA / 'tconstruct/loot_tables/entities/armor_stand.json').read_text(encoding='utf-8'))
    write_json(CONTENT_DATA / 'tconstruct/loot_table/entities/armor_stand.json', whole(upgrade_loot(table)))
    for name in SLIME_SPAWN_MODIFIERS:
        modifier = json.loads((GENERATED_DATA / 'tconstruct/forge/biome_modifier' / f'{name}.json').read_text(encoding='utf-8'))
        if modifier.get('type') != 'forge:add_spawns' or set(modifier) != {'type', 'biomes', 'spawners'}:
            raise SystemExit(f'biome modifier {name} is not a forge spawn modifier')
        biomes = modifier['biomes']
        if not isinstance(biomes, str):
            biomes = '#tconstruct:terracube_spawns' if name == 'spawn_terracube' else '#minecraft:is_overworld'
        rows = modifier['spawners'] if isinstance(modifier['spawners'], list) else [modifier['spawners']]
        spawners = [{'type': spawner['type'], 'weight': spawner['weight'], 'minCount': spawner['minCount'], 'maxCount': spawner['maxCount']} for spawner in rows]
        write_json(CONTENT_DATA / 'tconstruct/neoforge/biome_modifier' / f'{name}.json',
                   {'type': 'neoforge:add_spawns', 'biomes': biomes, 'spawners': whole(spawners)})
    return len(SLIME_ENTITIES) * 2 + len(SLIME_SPAWN_MODIFIERS) + 1


# Forge's convention tags do not exist in the host: the vanilla items they stand for in the recipes of the smeltery blocks.


# Forge's convention tags do not exist in the host: the items they stand for in the plain crafting recipes (the vanilla
# items for the vanilla metals, sand and glass; this port's declared items for the Tinkers metals and the slime balls),
# each read off the original's generated forge tags.
GLASS_ITEMS = ['minecraft:glass', 'minecraft:tinted_glass'] + [f'minecraft:{color}_stained_glass' for color in (
    'white', 'orange', 'magenta', 'light_blue', 'yellow', 'lime', 'pink', 'gray', 'light_gray', 'cyan', 'purple', 'blue', 'brown', 'green', 'red', 'black')]
FORGE_TAGS = {'forge:ingots/copper': ['minecraft:copper_ingot'], 'forge:ingots/netherite': ['minecraft:netherite_ingot'], 'forge:ingots/gold': ['minecraft:gold_ingot'], 'forge:ingots/iron': ['minecraft:iron_ingot'], 'forge:gems/quartz': ['minecraft:quartz'], 'forge:gems/diamond': ['minecraft:diamond'],
              'forge:gems/lapis': ['minecraft:lapis_lazuli'], 'forge:chests/wooden': ['minecraft:chest', 'minecraft:trapped_chest'],
              'forge:workbenches': ['minecraft:crafting_table', 'tconstruct:crafting_station'],
              'forge:sand/colorless': ['minecraft:sand'], 'forge:sand/red': ['minecraft:red_sand'], 'forge:glass': GLASS_ITEMS, 'forge:glass/colorless': ['minecraft:glass'], 'forge:glass/tinted': ['minecraft:tinted_glass'],
              'forge:dusts/redstone': ['minecraft:redstone'], 'forge:dusts/glowstone': ['minecraft:glowstone_dust'], 'forge:glass_panes/colorless': ['minecraft:glass_pane'],
              'forge:slimeball/earth': ['minecraft:slime_ball'], 'forge:slimeball/sky': ['tconstruct:sky_slime_ball'],
              'forge:slimeball/ichor': ['tconstruct:ichor_slime_ball'], 'forge:slimeball/ender': ['tconstruct:ender_slime_ball'],
              'forge:slimeballs': ['minecraft:slime_ball', 'tconstruct:sky_slime_ball', 'tconstruct:ichor_slime_ball', 'tconstruct:ender_slime_ball'],
              'forge:raw_materials/cobalt': ['tconstruct:raw_cobalt'], 'forge:raw_nuggets/knightmetal': ['tconstruct:knightmetal_shard'],
              'forge:nuggets/copper': ['tconstruct:copper_nugget'], 'forge:nuggets/netherite': ['tconstruct:netherite_nugget'],
              'forge:nuggets/netherite_scrap': ['tconstruct:debris_nugget'],
              'forge:string': ['minecraft:string'], 'forge:rods/wooden': ['minecraft:stick']}
# The shipped metals' own convention tags: each stands for the one declared item of its metal.
for _metal in METALS:
    FORGE_TAGS[f'forge:ingots/{_metal}'] = [f'tconstruct:{_metal}_ingot']
    FORGE_TAGS[f'forge:nuggets/{_metal}'] = [f'tconstruct:{_metal}_nugget']
    FORGE_TAGS[f'forge:storage_blocks/{_metal}'] = [f'tconstruct:{_metal}_block']
# Forge's dye convention: the tag for a colour stands for that colour's dye, which here is the
# vanilla dye. The clear stained glass dyeing recipes name nothing else undeclared. The anvil
# metal tag also nests the vanilla netherite storage block through Forge.
for _color in ('white', 'orange', 'magenta', 'light_blue', 'yellow', 'lime', 'pink', 'gray', 'light_gray',
               'cyan', 'purple', 'blue', 'brown', 'green', 'red', 'black'):
    FORGE_TAGS[f'forge:dyes/{_color}'] = [f'minecraft:{_color}_dye']
FORGE_TAGS['forge:storage_blocks/netherite'] = ['minecraft:netherite_block']
# The platform recipes name the vanilla nugget tags and the wood recipes the wooden rod tag: what they stand for here.
FORGE_TAGS['forge:nuggets/gold'] = ['minecraft:gold_nugget']
FORGE_TAGS['forge:nuggets/iron'] = ['minecraft:iron_nugget']
FORGE_TAGS['forge:rods/wooden'] = ['minecraft:stick']
# The necrotic bone stand's wither-bone tag: the original's generated forge tags name the necrotic
# bone alone (wither_bones is it plus bones/wither, which is it again).
FORGE_TAGS['forge:wither_bones'] = ['tconstruct:necrotic_bone']
CRAFTING_TYPES = ('minecraft:crafting_shaped', 'minecraft:crafting_shapeless', 'minecraft:smelting', 'minecraft:blasting', 'minecraft:stonecutting', 'mantle:crafting_shaped_retextured')
# Tags of the table recipes that stay tag ingredients (they hold vanilla items); the smeltery recipes expand tags to the declared items.
TABLE_TAGS = {'tconstruct:planklike', 'tconstruct:workstation_rock', 'tconstruct:seared_blocks', 'minecraft:planks', 'minecraft:logs'}
# Tags of the gadget recipes that stay tag ingredients: the port ships the tag and its members are vanilla items
# (tconstruct:bones is just minecraft:bone), which the expansion to declared items only would silently drop.
GADGET_TAGS = {'tconstruct:bones'}
CRAFTING_TARGET = CONTENT_DATA / 'tconstruct/recipe'


# The six books of TinkerCommons (UNSTACKABLE_PROPS items whose use opens the book of the same name, TinkerBookItem).
BOOK_ITEMS = ('materials_and_you', 'puny_smelting', 'mighty_smelting', 'tinkers_gadgetry', 'fantastic_foundry', 'encyclopedia')
BOOK_SOURCE = ROOT / '.local/tconstruct-source/src/main/resources/assets/tconstruct/book'


def write_book_declarations():
    """The book items: unstackable plain items; opening the book is books.rs' work, the data is copy_book_assets'."""
    for name in BOOK_ITEMS:
        write_json(HORIZON_DECLARATIONS / 'item' / f'{name}.json', {'template': 'minecraft:paper', 'max_stack_size': 1}, compact=True)
    return len(BOOK_ITEMS)


def repair_lenient_json(text):
    """The original's book files are read by the game through Gson's lenient parser and use spellings strict JSON
    forbids: hexadecimal numbers (`"slotColor": 0xFDF500` in every appearance file), trailing commas and missing
    commas between values (a few localized pages), a missing closing brace (one page), and one bare unquoted token
    (a stray hash in one page's text list). The repair works on the token stream, so string contents are never
    touched: a hex literal becomes its decimal value, a bare token that is no number or literal becomes a quoted
    string (what Gson makes of it), a comma is inserted where a value ends and the next begins, a trailing comma is
    dropped, and a closing bracket first closes the scopes left open inside it. The values are unchanged."""
    tokens, i = [], 0
    while i < len(text):
        ch = text[i]
        if ch in ' \t\r\n':
            j = i
            while j < len(text) and text[j] in ' \t\r\n':
                j += 1
            tokens.append(('space', text[i:j]))
            i = j
        elif ch in '{}[]:,':
            tokens.append((ch, ch))
            i += 1
        elif ch == '"':
            j = i + 1
            while j < len(text):
                if text[j] == '\\':
                    j += 2
                    continue
                if text[j] == '"':
                    j += 1
                    break
                j += 1
            tokens.append(('string', text[i:j]))
            i = j
        else:
            j = i
            while j < len(text) and text[j] not in ' \t\r\n{}[]:,"':
                j += 1
            tokens.append(('atom', text[i:j]))
            i = j
    number = re.compile(r'-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?$')
    out, stack, prev = [], [], None
    for index, (kind, raw) in enumerate(tokens):
        if kind == 'space':
            out.append(raw)
            continue
        if kind == 'atom':
            if re.fullmatch(r'-?0[xX][0-9a-fA-F]+', raw):
                raw = str(int(raw, 16))
            elif number.match(raw) or raw in ('true', 'false', 'null'):
                pass
            else:
                raw = json.dumps(raw, ensure_ascii=False)
                kind = 'string'
        if kind == ',':
            following = next((k for k, _ in tokens[index + 1:] if k != 'space'), None)
            if following in ('}', ']', None):
                continue
        if kind in ('{', '[', 'string', 'atom') and prev == 'value':
            out.append(',')
        if kind in ('{', '['):
            stack.append(kind)
        elif kind in ('}', ']'):
            while stack and (stack[-1] == '{') != (kind == '}'):
                out.append('}' if stack.pop() == '{' else ']')
            if stack:
                stack.pop()
        out.append(raw)
        prev = 'value' if kind in ('string', 'atom', '}', ']') else kind
    while stack:
        out.append('}' if stack.pop() == '{' else ']')
    return ''.join(out)


def copy_book_assets():
    """The books' Mantle data (each book's index, appearance, section files, per-language pages and language tables, and the
    shared images), copied whole: books.rs walks the tree through the client resources in the original's layout, so nothing
    is selected or reshaped. The JSON files are normalized on the way (a leading BOM and the line endings, as in copy_lang,
    and the Gson leniencies repair_lenient_json names — the values are unchanged, and each file must parse as strict JSON
    afterwards or the import fails); the images and the two structure templates are binary."""
    target = ASSET_TARGET / 'book'
    shutil.rmtree(target, ignore_errors=True)
    count = 0
    for path in sorted(BOOK_SOURCE.rglob('*')):
        if not path.is_file():
            continue
        destination = target / path.relative_to(BOOK_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        if path.suffix == '.json':
            text = repair_lenient_json(path.read_bytes().removeprefix(b'\xef\xbb\xbf').decode('utf-8').replace('\r\n', '\n'))
            json.loads(text)
            destination.write_bytes(text.encode('utf-8'))
        elif path.suffix == '.lang':
            destination.write_bytes(path.read_bytes().removeprefix(b'\xef\xbb\xbf').replace(b'\r\n', b'\n'))
        else:
            shutil.copyfile(path, destination)
        count += 1
    return count


def copy_crafting_recipes():
    """The plain crafting, smelting, blasting and stonecutting recipes of the original's `smeltery/`, `tables/`, `common/`,
    `world/`, `gadgets/` and `compat/` recipe folders (the smeltery's structure blocks and periphery, the stations and
    their tables, the commons: the metals' compaction, the clear glass, the slime balls and their blocks, the platforms,
    the cheese and the firewood; the wood families under `world/wood`; the gadgets whose recipes name nothing undeclared
    -- punji, the ichor and magma cakes, and the fancy frames and armor stands) in the game's recipe format, under the
    original's relative path as the recipe id: ingredients and results are plain ids (`#` for a tag), forge's convention
    tags become the items they stand for, vanilla's own tags stay tags, a `forge:difference` ingredient is expanded over
    the original's tag, and the melter's `tconstruct:no_container` gauge match becomes the two gauges. A recipe naming
    an item this port does not declare is left out and returned. The four shapeless book recipes of the commons ride along,
    now that the books are declared (the other two books are casting recipes, copied with the fluid recipes). The `compat/` folder
    holds no plain recipes at all: its melting and casting recipes are gated on other mods (`forge:mod_loaded`), which this
    port does not carry. The `tconstruct:crafting_shaped_materials` type (the material anvils and forges, the travelers
    armor, the fake ingot conversion) is not converted either: it resolves its material ingredient against the material
    registry at craft time, which no plain recipe consumer here does, so its recipes wait with the stations. A recipe's
    `tconstruct:config` conditions ship in the original's form (see the note at the write); every other condition
    type stays stripped."""
    shutil.rmtree(CRAFTING_TARGET, ignore_errors=True)
    declared = {f'tconstruct:{p.stem}' for kind in ('block', 'item') for p in (HORIZON_DECLARATIONS / kind).glob('*.json')}
    tag_files = GENERATED_DATA / 'tconstruct/tags/items'

    def known(item):
        return item in declared or item.startswith('minecraft:')

    def forge_members(tag, seen=()):
        """The items a Forge convention tag names in the original's own tag providers (its metal storage blocks are
        Tinkers blocks), or the vanilla items of the convention table when Forge itself generated the tag (the vanilla
        metals). Anything else is unknown and stops the recipe, as an undeclared item does."""
        path = tag.split(':', 1)[1]
        file = GENERATED_DATA / 'forge/tags/items' / f'{path}.json'
        if tag in seen:
            raise KeyError(tag)
        if file.is_file():
            entries = json.loads(file.read_text(encoding='utf-8'))['values']
        elif path in VANILLA_CONVENTION:
            entries = VANILLA_CONVENTION[path]
        else:
            raise KeyError(tag)
        result = []
        for entry in entries:
            entry = entry if isinstance(entry, str) else entry['id']
            if entry.startswith('#forge:'):
                result += forge_members(entry[1:], seen + (tag,))
            elif entry.startswith('#'):
                result += members(entry[1:], seen + (tag,))
            elif entry in declared or entry.startswith('minecraft:'):
                result.append(entry)
        return result

    def members(tag, seen=()):
        namespace, path = tag.split(':', 1)
        if namespace != 'tconstruct' or tag in seen:
            raise KeyError(tag)
        result = []
        for entry in json.loads((tag_files / f'{path}.json').read_text(encoding='utf-8'))['values']:
            entry = entry if isinstance(entry, str) else entry['id']
            if entry.startswith('#') and entry[1:].startswith('forge:'):
                # A tag nested through a Forge tag: what the Forge tag stands for here, no more.
                # One the port does not carry stands for nothing (the original marks those entries
                # optional: they are other mods' metals).
                result += [item for item in FORGE_TAGS.get(entry[1:], []) if known(item)]
            elif entry.startswith('#'):
                result += members(entry[1:], seen + (tag,))
            elif entry in declared:
                result.append(entry)
        return result

    tables = False

    def convert(node):
        """An ingredient as a string or a list of strings; KeyError when it names something this port has not."""
        if isinstance(node, list):
            out = []
            for part in node:
                part = convert(part)
                out += part if isinstance(part, list) else [part]
            return out
        kind = node.get('type')
        if kind == 'forge:difference':
            base, taken = convert(node['base']), convert(node['subtracted'])
            base = base if isinstance(base, list) else [base]
            taken = taken if isinstance(taken, list) else [taken]
            if any(entry.startswith('#') for entry in base + taken):
                raise KeyError('difference of tags')
            return [entry for entry in base if entry not in taken]
        if kind == 'tconstruct:no_container':
            return convert(node['match'])
        if kind is not None:
            raise KeyError(kind)
        if 'tag' in node:
            tag = node['tag']
            if (tables and tag in TABLE_TAGS) or (gadgets and tag in GADGET_TAGS):
                return '#' + tag
            if tables and tag == 'forge:rods/wooden':
                return 'minecraft:stick'
            if tag in FORGE_TAGS:
                return list(FORGE_TAGS[tag])
            if tag.startswith('minecraft:'):
                return '#' + tag
            if tag.startswith('tconstruct:'):
                return members(tag)
            raise KeyError(tag)
        if not known(node['item']):
            raise KeyError(node['item'])
        return node['item']

    def one(ingredient):
        ingredient = convert(ingredient)
        return ingredient

    written, left_out = 0, []
    for path in sorted((FLUID_RECIPE_SOURCE / 'smeltery').rglob('*.json')) + sorted((FLUID_RECIPE_SOURCE / 'tables').rglob('*.json')) + sorted((FLUID_RECIPE_SOURCE / 'common').rglob('*.json')) + sorted((FLUID_RECIPE_SOURCE / 'world').rglob('*.json')) + sorted((FLUID_RECIPE_SOURCE / 'gadgets').rglob('*.json')) + sorted((FLUID_RECIPE_SOURCE / 'compat').rglob('*.json')) + sorted((FLUID_RECIPE_SOURCE / 'tools/building').rglob('*.json')):
        tables = path.is_relative_to(FLUID_RECIPE_SOURCE / 'tables')
        gadgets = path.is_relative_to(FLUID_RECIPE_SOURCE / 'gadgets')
        data = json.loads(path.read_text(encoding='utf-8'))
        if data.get('type') == 'forge:conditional':
            # The kiln pair (smeltery/seared, smeltery/scorched): the ceramics entry waits on that mod, and
            # the blasting fallback is gated on forge:true, so the fallback is the recipe this port's world
            # has; it ships as the plain blasting recipe it wraps.
            fallback = [entry['recipe'] for entry in data.get('recipes', [])
                        if entry.get('conditions') == [{'type': 'forge:true'}] and entry['recipe'].get('type') in CRAFTING_TYPES]
            if len(fallback) != 1:
                continue
            data = fallback[0]
        # the retextured table and smeltery recipes ship as plain shaped recipes, but they keep their
        # `texture` and `match_all`: the game's shaped parser ignores them and the port's consumer
        # (api/src/retextured.rs) reads them to put the texture on the result
        if data.get('type') not in CRAFTING_TYPES or (data['type'] == 'mantle:crafting_shaped_retextured'
                                                      and not (tables or path.is_relative_to(FLUID_RECIPE_SOURCE / 'smeltery'))):
            continue
        stem = path.relative_to(FLUID_RECIPE_SOURCE).with_suffix('').as_posix()
        result = data['result']
        item = result['item'] if isinstance(result, dict) else result
        count = result.get('count', 1) if isinstance(result, dict) else data.get('count', 1)
        try:
            if data['type'] in ('minecraft:crafting_shaped', 'mantle:crafting_shaped_retextured'):
                out = {'type': 'minecraft:crafting_shaped', 'key': {key: one(value) for key, value in data['key'].items()}, 'pattern': data['pattern']}
                if data['type'] == 'mantle:crafting_shaped_retextured':
                    out['texture'] = data['texture']
                    out['match_all'] = data['match_all']
            elif data['type'] == 'minecraft:crafting_shapeless':
                out = {'type': data['type'], 'ingredients': [one(value) for value in data['ingredients']]}
            elif data['type'] in ('minecraft:smelting', 'minecraft:blasting'):
                out = {'type': data['type'], 'cookingtime': data['cookingtime'], 'experience': data['experience'], 'ingredient': one(data['ingredient'])}
            else:
                out = {'type': data['type'], 'ingredient': one(data['ingredient'])}
        except KeyError as missing:
            left_out.append((stem, str(missing)))
            continue
        if any(value == [] for value in out.get('key', {}).values()) or out.get('ingredient') == [] or any(value == [] for value in out.get('ingredients', [])):
            left_out.append((stem, 'empty ingredient'))
            continue
        if 'group' in data:
            out['group'] = data['group']
        out['result'] = {'id': item} if count == 1 else {'id': item, 'count': count}
        if not known(item):
            left_out.append((stem, item))
            continue
        config_conditions = [entry for entry in data.get('conditions', []) if entry.get('type') == 'tconstruct:config']
        if config_conditions:
            # A `tconstruct:config` condition ships in the original's form, ahead of the body as the original
            # writes it. The host reads only `neoforge:conditions`, so the key is inert in the file: the gate
            # is enforced at match time by the consumer's crafting-match veto (api/src/recipe_conditions.rs),
            # which decides from the same prop names in the parsed common config.
            out = {'type': out['type'], 'conditions': config_conditions, **{key: value for key, value in out.items() if key != 'type'}}
        write_json(CRAFTING_TARGET / f'{stem}.json', out)
        written += 1
    # The shaped-materials recipes (the material anvils and forges in `tables/`, the travellers armor in
    # `tools/`, the fake ingot's storage block): the type resolves its material ingredients against the
    # material registry at craft time, which no plain conversion can express, so the key's material
    # ingredients ship verbatim for the port's consumer (api/src/shaped_materials.rs) while the rest of
    # the recipe converts like any shaped recipe. `parts` and `extra_materials` ship verbatim too: the
    # game's shaped parser ignores them and the consumer reads them.
    for path in sorted(FLUID_RECIPE_SOURCE.rglob('*.json')):
        data = json.loads(path.read_text(encoding='utf-8'))
        if data.get('type') != 'tconstruct:crafting_shaped_materials':
            continue
        tables = path.is_relative_to(FLUID_RECIPE_SOURCE / 'tables')
        gadgets = False
        stem = path.relative_to(FLUID_RECIPE_SOURCE).with_suffix('').as_posix()
        result = data['result']
        item = result['item'] if isinstance(result, dict) else result
        count = result.get('count', 1) if isinstance(result, dict) else data.get('count', 1)
        def material_or_converted(value):
            if isinstance(value, list):
                return [material_or_converted(part) for part in value]
            if isinstance(value, dict) and value.get('type') in ('tconstruct:material', 'tconstruct:material_value'):
                named = value.get('item')
                if named is not None and not known(named):
                    raise KeyError(named)
                return value
            return one(value)
        try:
            key = {symbol: material_or_converted(value) for symbol, value in data['key'].items()}
        except KeyError as missing:
            left_out.append((stem, str(missing)))
            continue
        out = {'type': 'minecraft:crafting_shaped', 'key': key, 'pattern': data['pattern'], 'parts': data['parts']}
        if 'extra_materials' in data:
            out['extra_materials'] = data['extra_materials']
        if 'group' in data:
            out['group'] = data['group']
        out['result'] = {'id': item} if count == 1 else {'id': item, 'count': count}
        if not known(item):
            left_out.append((stem, item))
            continue
        write_json(CRAFTING_TARGET / f'{stem}.json', out)
        written += 1
    return written, left_out


def copy_vanilla_tags():
    """The original's additions to vanilla's block tags (mineable/*, needs_*_tool, enderman_holdable...), reduced to the
    declared blocks and to the Tinkers block tags that ship (the host drops unknown ids anyway; this keeps the files to what this
    port registers, and a later unit's blocks join on the next import). Entries of any other namespace stop the import,
    Forge's convention tags are left out (that namespace does not exist here, as in copy_tags). `forge:needs_gold_tool` has no
    26.3 tag: gold tools are wooden ones there, and no world block needs the tool to drop anyway. The beacon base
    tag names its metals through Forge's storage-block tags: a shipped metal's tag stands for its block, as
    forge_members resolves them for the recipes. The item tags (tool families, logs...) and the entity-type
    tags join with the same reduction: an entry ships when it names a declared item, a shipped Tinkers tag or
    a registered entity type; anything else (Forge's namespace, vanilla's own entries, which the base game
    already carries) is left out. The gold tier (forge:needs_gold_tool, which has no 26.3 needs_* tag) is rebuilt from the tool components' incorrect_for_* tags at the tail of this function."""
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
            if value.startswith('#forge:storage_blocks/') and f"{value[len('#forge:storage_blocks/'):]}_block" in declared:
                values.append(f"tconstruct:{value[len('#forge:storage_blocks/'):]}_block")
                continue
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
    # SlimeDirtBlock sustains PLAINS. VegetationBlock checks this ground tag in 26.3.
    write_json(target / 'supports_vegetation.json', {'values': [f'tconstruct:{dirt}_slime_dirt' for dirt in DIRT_TYPES]})
    count += 1
    # The gold tier (forge:needs_gold_tool): 26.3 has no needs_* tag for it, but the tier survives in the
    # tool components - a wooden tool refuses what minecraft:incorrect_for_wooden_tool names, a gold tool
    # what minecraft:incorrect_for_gold_tool names. Naming the gold-tier blocks that require a correct
    # tool for drops under the wooden tag alone reproduces the tier exactly: wooden tools fail, gold and
    # better still harvest. The members are the original's needs_gold_tool tag minus sky_slime_dirt,
    # which stays out on purpose: SlimeDirtBlock sets no requiresCorrectToolForDrops, so its membership
    # would be inert here as it is in the original. The sky members are TinkerWorld's (the sky slime
    # grass, the skyroot log and wood); the seared and scorched members are TinkerSmeltery's building
    # blocks, tanks, gauges and devices, whose block suppliers all carry requiresCorrectToolForDrops.
    write_json(target / 'incorrect_for_wooden_tool.json', {'values': [
        'tconstruct:blood_sky_slime_grass', 'tconstruct:sky_sky_slime_grass', 'tconstruct:sky_vanilla_slime_grass', 'tconstruct:skyroot_log',
        'tconstruct:skyroot_wood', 'tconstruct:seared_stone', 'tconstruct:seared_stone_slab', 'tconstruct:seared_stone_stairs',
        'tconstruct:seared_paver', 'tconstruct:seared_paver_slab', 'tconstruct:seared_paver_stairs', 'tconstruct:seared_cobble',
        'tconstruct:seared_cobble_slab', 'tconstruct:seared_cobble_stairs', 'tconstruct:seared_cobble_wall', 'tconstruct:seared_bricks',
        'tconstruct:seared_bricks_slab', 'tconstruct:seared_bricks_stairs', 'tconstruct:seared_bricks_wall', 'tconstruct:seared_cracked_bricks',
        'tconstruct:seared_fancy_bricks', 'tconstruct:seared_triangle_bricks', 'tconstruct:seared_ladder', 'tconstruct:seared_lamp',
        'tconstruct:seared_glass', 'tconstruct:seared_soul_glass', 'tconstruct:seared_tinted_glass', 'tconstruct:seared_glass_pane',
        'tconstruct:seared_soul_glass_pane', 'tconstruct:scorched_bricks', 'tconstruct:scorched_bricks_slab', 'tconstruct:scorched_bricks_stairs',
        'tconstruct:scorched_bricks_fence', 'tconstruct:scorched_road', 'tconstruct:scorched_road_slab', 'tconstruct:scorched_road_stairs',
        'tconstruct:scorched_stone', 'tconstruct:polished_scorched_stone', 'tconstruct:chiseled_scorched_bricks', 'tconstruct:scorched_ladder',
        'tconstruct:scorched_lamp', 'tconstruct:scorched_glass', 'tconstruct:scorched_soul_glass', 'tconstruct:scorched_tinted_glass',
        'tconstruct:scorched_glass_pane', 'tconstruct:scorched_soul_glass_pane', 'tconstruct:seared_fuel_tank', 'tconstruct:seared_fuel_gauge',
        'tconstruct:seared_ingot_tank', 'tconstruct:seared_ingot_gauge', 'tconstruct:scorched_fuel_tank', 'tconstruct:scorched_fuel_gauge',
        'tconstruct:scorched_ingot_tank', 'tconstruct:scorched_ingot_gauge', 'tconstruct:seared_lantern', 'tconstruct:seared_faucet',
        'tconstruct:seared_channel', 'tconstruct:seared_basin', 'tconstruct:seared_table', 'tconstruct:seared_casting_tank',
        'tconstruct:scorched_lantern', 'tconstruct:scorched_faucet', 'tconstruct:scorched_channel', 'tconstruct:scorched_basin',
        'tconstruct:scorched_table', 'tconstruct:seared_heater', 'tconstruct:seared_melter', 'tconstruct:scorched_alloyer']})
    count += 1
    # The declared items, and the block items with them (a block declaration's `item` flag): the wood
    # families, the signs and the heads are named by the item tags through their blocks.
    declared_items = {p.stem for p in (HORIZON_DECLARATIONS / 'item').glob('*.json')}
    for path in (HORIZON_DECLARATIONS / 'block').glob('*.json'):
        if json.loads(path.read_text(encoding='utf-8')).get('item'):
            declared_items.add(path.stem)
    shipped_item_tags = {path.relative_to(TAG_TARGET / 'item').with_suffix('').as_posix() for path in (TAG_TARGET / 'item').rglob('*.json')}
    item_target = CONTENT_DATA / 'minecraft/tags/item'
    shutil.rmtree(item_target, ignore_errors=True)
    for path in sorted((GENERATED_DATA / 'minecraft/tags/items').rglob('*.json')):
        if path.stem == 'arrows':
            # The arrows tag's one member this port declares is the arrow tool itself. A shipped vanilla
            # tag file stands for the whole tag in the port's item snapshot, so the file would leave the
            # host's arrows tag holding nothing but a tool, and a tool in a casting table is not a cast;
            # the member returns when the snapshot merges the host's own tag members.
            continue
        values = []
        for entry in json.loads(path.read_text(encoding='utf-8')).get('values', []):
            value = entry if isinstance(entry, str) else entry['id']
            if value.startswith('#forge:ingots/') and f"{value[len('#forge:ingots/'):]}_ingot" in declared_items:
                # the beacon payment tag names its metals through Forge's ingot tags, as the beacon
                # base tag names them through the storage-block tags
                values.append(f"tconstruct:{value[len('#forge:ingots/'):]}_ingot")
                continue
            if value.startswith('#tconstruct:'):
                keep = value[len('#tconstruct:'):] in shipped_item_tags
            elif value.startswith('tconstruct:'):
                keep = value[len('tconstruct:'):] in declared_items
            else:
                keep = False
            if keep:
                values.append(entry)
        if values:
            write_json(item_target / path.relative_to(GENERATED_DATA / 'minecraft/tags/items'), {'values': values})
            count += 1
    entity_target = CONTENT_DATA / 'minecraft/tags/entity_type'
    shutil.rmtree(entity_target, ignore_errors=True)
    for path in sorted((GENERATED_DATA / 'minecraft/tags/entity_types').rglob('*.json')):
        values = []
        for entry in json.loads(path.read_text(encoding='utf-8')).get('values', []):
            value = entry if isinstance(entry, str) else entry['id']
            if value.startswith('tconstruct:') and value[len('tconstruct:'):] in registered_entity_types():
                values.append(entry)
        if values:
            write_json(entity_target / path.relative_to(GENERATED_DATA / 'minecraft/tags/entity_types'), {'values': values})
            count += 1
    return count


def copy_cobalt_tags():
    """Project just U1's members and nested tag references from Forge's generated tags into c:."""
    for kind in ('item', 'block'):
        paths = ['ores', 'ores/cobalt', 'storage_blocks', 'storage_blocks/raw_cobalt',
                 'ore_rates/singular', 'ores_in_ground/netherrack']
        if kind == 'item':
            paths += ['raw_materials', 'raw_materials/cobalt']
        for path in paths:
            src = GENERATED_DATA / f'forge/tags/{kind}s/{path}.json'
            entries = json.loads(src.read_text(encoding='utf-8'))['values']
            kept = [entry.replace('#forge:', '#c:') for entry in entries
                    if entry in ('tconstruct:cobalt_ore', 'tconstruct:raw_cobalt', 'tconstruct:raw_cobalt_block')
                    or entry.removeprefix('#forge:') in paths]
            if not kept:
                raise ValueError(f'{src}: missing cobalt membership')
            write_json(CONTENT_DATA / f'c/tags/{kind}/{path}.json', {'values': kept})


def copy_hidden_fluid_tag():
    """The original's common fluid tag (its data/c/tags/fluids/hidden_from_recipe_viewers.json):
    the fluids recipe viewers hide, which tconstruct's own hide_in_creative_tanks fluid tag
    references. The port's tag kinds are singular, so it lands in c/tags/fluid. The values are
    the original's, verbatim: just molten soulsteel."""
    src = GENERATED_DATA / 'c/tags/fluids/hidden_from_recipe_viewers.json'
    entries = json.loads(src.read_text(encoding='utf-8'))['values']
    write_json(CONTENT_DATA / 'c/tags/fluid/hidden_from_recipe_viewers.json', {'values': entries})


LANG_SOURCE = ROOT / '.local/tconstruct-source/src/main/resources/assets/tconstruct/lang'
LANG_TARGET = ROOT / 'tconstruct/content/assets/tconstruct/lang'
LANGUAGES = ['de_de', 'en_us', 'es_ar', 'es_cl', 'es_es', 'es_mx', 'fr_fr', 'id_id', 'it_it', 'ja_jp', 'ko_kr', 'lzh', 'pl_pl', 'pt_br', 'ru_ru', 'sv_se', 'tr_tr', 'uk_ua', 'zh_cn', 'zh_tw']


def copy_lang():
    """The original's translations for the languages this port ships (modifier, block, item,
    material and gui names): the content mod's own lang files, read by the game and by PlutoLib's I18n. Line endings are normalised to LF and a leading UTF-8 BOM is dropped (the original's lzh file has one; language files are parsed as strict UTF-8 JSON)."""
    if LANG_TARGET.exists():
        shutil.rmtree(LANG_TARGET)
    LANG_TARGET.mkdir(parents=True)
    for language in LANGUAGES:
        # Only the line endings are normalized (the original checkout may be CRLF) and a leading BOM dropped; the text is the original's.
        data = (LANG_SOURCE / f'{language}.json').read_bytes().removeprefix(b'\xef\xbb\xbf').replace(b'\r\n', b'\n')
        (LANG_TARGET / f'{language}.json').write_bytes(data)
    return len(LANGUAGES)


def copy_sounds():
    """The original's sound definitions and files (assets/tconstruct/sounds.json and sounds/*.ogg), byte for
    byte: the sound events the port's code plays by name (the bonk, the sling, the little saw) resolve through
    this table, and the .ogg files are the recordings themselves."""
    target = ASSET_TARGET / 'sounds'
    if target.exists():
        shutil.rmtree(target)
    target.mkdir(parents=True)
    count = 0
    for path in sorted((ASSET_SOURCE / 'sounds').glob('*.ogg')):
        shutil.copyfile(path, target / path.name)
        count += 1
    # sounds.json ships with LF endings like every other JSON here (the original's file is CRLF).
    (ASSET_TARGET / 'sounds.json').write_bytes((ASSET_SOURCE / 'sounds.json').read_bytes().replace(b'\r\n', b'\n'))
    return count + 1


MOB_EQUIPMENT_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/tinkering/mob_equipment'
MOB_EQUIPMENT_TARGET = TARGET / 'mob_equipment'


def copy_mob_equipment():
    """The mob equipment sets (which mobs spawn holding Tinkers gear), byte for byte, load conditions and all:
    conditions are data the loader evaluates, as with the integration materials. The consumer that equips
    spawning mobs reads this folder."""
    if MOB_EQUIPMENT_TARGET.exists():
        shutil.rmtree(MOB_EQUIPMENT_TARGET)
    count = 0
    for path in sorted(MOB_EQUIPMENT_SOURCE.rglob('*.json')):
        destination = MOB_EQUIPMENT_TARGET / path.relative_to(MOB_EQUIPMENT_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    return count



# --- Advancements: the 50 gameplay advancements and their grant functions -------------------
ADVANCEMENT_GROUPS = ['foundry', 'internal', 'smeltery', 'tools', 'world']
ADVANCEMENT_KEYS = {'parent', 'display', 'criteria', 'requirements', 'rewards', 'sends_telemetry_event'}
ADVANCEMENT_DISPLAY_KEYS = {'icon', 'title', 'description', 'frame', 'background', 'show_toast', 'announce_to_chat', 'hidden'}


def upgrade_advancement(data, name):
    """A 1.20.1 advancement in the host's form: the display icon names its item under `id`
    (the host reads no other icon field, so the icon's NBT is dropped); criteria, requirements
    and rewards are already the shapes the host's parser reads. Unreviewed keys stop the import."""
    if not isinstance(data, dict) or set(data) - ADVANCEMENT_KEYS:
        raise SystemExit(f'advancement {name}: unreviewed keys')
    result = dict(data)
    display = result.get('display')
    if display is not None:
        if not isinstance(display, dict) or set(display) - ADVANCEMENT_DISPLAY_KEYS:
            raise SystemExit(f'advancement {name}: unreviewed display keys')
        display = dict(display)
        icon = display.get('icon')
        if icon is not None:
            if not isinstance(icon, dict) or set(icon) - {'item', 'nbt'} or 'item' not in icon:
                raise SystemExit(f'advancement {name}: unreviewed icon {icon!r}')
            display['icon'] = {'id': icon['item']}
        result['display'] = display
    return result


def unwrap_conditional(data, name):
    """A Forge conditional advancement wrapper in the form the loader presents in this port's world:
    the entry gated on forge:true (the fallback that always holds), or the starting book's entry, whose
    spawn_with_book config defaults to true and which the host's advancement loading cannot gate.
    A wrapper whose entries all wait on another mod's presence has no form here and returns None."""
    if 'advancements' not in data:
        return data
    entries = data['advancements']
    for entry in entries:
        if entry.get('conditions') == [{'type': 'forge:true'}]:
            return entry['advancement']
    if len(entries) == 1 and entries[0].get('conditions') == [{'type': 'tconstruct:config', 'prop': 'spawn_with_book'}]:
        return entries[0]['advancement']
    if all(any(condition.get('type') == 'forge:mod_loaded' for condition in entry.get('conditions', [])) for entry in entries):
        return None
    raise SystemExit(f'advancement {name}: unreviewed conditional wrapper')


def copy_advancements():
    """The gameplay advancements (data/<ns>/advancement, the host's folder name), converted by
    upgrade_advancement, and the recipe unlock advancements of the recipes/ tree with them: the
    host's advancement engine evaluates minecraft:recipe_unlocked (its own advancements carry
    it), so the unlocks drive the recipe book as the original's do. Forge conditional wrappers
    ship unwrapped in the form the loader presents in this port's world (unwrap_conditional)."""
    source_root = GENERATED_DATA / 'tconstruct/advancements'
    target_root = CONTENT_DATA / 'tconstruct/advancement'
    shutil.rmtree(target_root, ignore_errors=True)
    count = 0
    for group in ADVANCEMENT_GROUPS:
        for entry in sorted((source_root / group).glob('*.json')):
            data = unwrap_conditional(json.loads(entry.read_text(encoding='utf-8')), entry.name)
            if data is None:
                raise SystemExit(f'advancement {entry.name}: a gameplay advancement waits on another mod')
            write_json(target_root / group / entry.name, upgrade_advancement(data, entry.name))
            count += 1
    skipped = []
    for entry in sorted((source_root / 'recipes').rglob('*.json')):
        data = unwrap_conditional(json.loads(entry.read_text(encoding='utf-8')), entry.name)
        if data is None:
            skipped.append(entry.relative_to(source_root / 'recipes').with_suffix('').as_posix())
            continue
        write_json(target_root / 'recipes' / entry.relative_to(source_root / 'recipes'), upgrade_advancement(data, entry.name))
        count += 1
    if skipped:
        print(f'advancements left out (they wait on another mod): {skipped}')
    return count


def copy_advancement_functions():
    """The grant_advancement functions the internal advancements reward, byte for byte
    (data/<ns>/function, the host's folder name). Each grants one vanilla advancement."""
    source_root = GENERATED_DATA / 'tconstruct/functions/grant_advancement'
    target_root = CONTENT_DATA / 'tconstruct/function/grant_advancement'
    shutil.rmtree(target_root, ignore_errors=True)
    count = 0
    for entry in sorted(source_root.glob('*.mcfunction')):
        destination = target_root / entry.name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(entry, destination)
        count += 1
    return count


def copy_gameplay_loot():
    """The starting book's reward table: an advancement_reward table giving the book item."""
    source = GENERATED_DATA / 'tconstruct/loot_tables/gameplay/starting_book.json'
    table = whole(upgrade_loot(json.loads(source.read_text(encoding='utf-8'))))
    write_json(CONTENT_DATA / 'tconstruct/loot_table/gameplay/starting_book.json', table)
    return 1


def main():
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    head = subprocess.run(['git', '-C', str(ROOT / '.local/tconstruct-source'), 'rev-parse', 'HEAD'],
                          capture_output=True, text=True, check=True).stdout.strip()
    expected = pins.get('tconstruct_revision') or pins.get('java_revision') or head
    if head != expected:
        raise SystemExit(f'original checkout {head} differs from the pinned {expected}')
    # No material is excluded: a definition's condition is data the loader evaluates,
    # not an import-time filter, so the sixteen integration materials ship byte for byte
    # with the other eighty-eight and simply never select in a vanilla-only world.
    skipped_materials = set()
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
    write_convention_tags()
    fluid_count, fluid_names = write_fluid_defs()
    write_render_data()
    tags = copy_tags() + copy_damage_tags() + copy_fluid_host_tags(fluid_names)
    copy_enchantment_data()
    write_creative_fluids(fluid_names)
    tags += write_block_tag_ingredients()
    write_damage_vanilla()
    write_instrument_tags()
    languages = copy_lang()
    sounds = copy_sounds()
    mob_equipment = copy_mob_equipment()
    fuels = write_vanilla_fuels()
    world_blocks_written, world_items_written = write_world_declarations()
    write_gadget_declarations()
    write_book_declarations()
    book_files = copy_book_assets()
    crafting_written, crafting_left = copy_crafting_recipes()
    models, textures, unrendered = copy_client_assets()
    fluid_blocks = write_fluid_world()
    fluid_textures = copy_fluid_textures()
    copy_modifier_icons()
    trim_palettes = copy_trim_palettes()
    colormaps = copy_colormaps()
    mob_effect_icons = copy_mob_effect_icons()
    write_fluid_atlas()
    write_copper_can()
    transfers = write_fluid_transfers()
    world_loot = copy_world_loot(sorted({*world_blocks(), *world_geode_blocks(), *smeltery_blocks(), *gadget_blocks()} | {'cast_chest', 'crafting_station', 'fake_storage_block', 'part_chest', 'scorched_anvil', 'tinkers_anvil', 'tinkers_chest'}))
    world_worldgen = copy_world_features()
    write_slime_entities()
    world_tags = copy_vanilla_tags()
    copy_cobalt_tags()
    copy_hidden_fluid_tag()
    part_names, part_models, part_textures = copy_part_assets()
    tool_names, tool_models, tool_textures, tools_left_out = copy_tool_assets()
    worn_textures, worn_rows = copy_armor_worn_assets()
    advancements = copy_advancements()
    advancement_functions = copy_advancement_functions()
    gameplay_loot = copy_gameplay_loot()
    models, textures = len(list((ASSET_TARGET / 'models').rglob('*.json'))), len(list((ASSET_TARGET / 'textures').rglob('*.png')))
    unrendered = [name for name in unrendered if name not in part_names + tool_names and not name.endswith('_bucket') and name != 'copper_can']  # fluid buckets and the can are modelled by write_fluid_world and write_copper_can
    recipes = copy_material_recipes(skipped_materials) + copy_modifier_recipes() + copy_part_builder_recipes() + copy_recycling_recipes() + copy_severing_recipes() + copy_loot_modifiers() + copy_loot_injectors() + copy_fluid_recipes() + copy_fluid_tags() + copy_fluid_effects() + copy_damageable_recycling_recipes() + copy_damaging_recipes() + copy_table_recipes() + copy_building_recipes()
    layouts = copy_station_layouts()
    print(f'copied {copied} files, {tags} tags, {languages} language files and {recipes} material recipes from {head}; {layouts} station layouts; {models} models and {textures} textures (left out, need the original tool loader: {len(unrendered)}; tools with no layered item definition: {tools_left_out}); {worn_textures} worn armor textures and {worn_rows} material variants; {len(types)} modifier module types; {fuels} vanilla fuels; {fluid_blocks} world fluids with a bucket; {fluid_textures} fluid textures; {transfers} fluid container transfers; {crafting_written} crafting/smelting/stonecutting recipes (left out for an undeclared ingredient: {crafting_left}); excluded materials: {sorted(skipped_materials)}; {advancements} advancements with {advancement_functions} grant functions and {gameplay_loot} gameplay loot table; {sounds} sound files, {mob_equipment} mob equipment files')


if __name__ == '__main__':
    sys.exit(main())
