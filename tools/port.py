#!/usr/bin/env python3
"""Clean-product boundary, deterministic work queue and development packaging.

Structural checks are not proof of native linkage or Java parity.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = Path('tconstruct/mixins/tconstruct')
CONTENT = Path('tconstruct/content')
# Content-mod inputs are discovered on disk but each must match one reviewed shape.
CONTENT_PATHS = (
    re.compile(r'horizon-mod\.toml'), re.compile(r'assets/tconstruct/lang/(?:de_de|en_us|es_ar|es_cl|es_es|es_mx|fr_fr|id_id|it_it|ja_jp|ko_kr|lzh|pl_pl|pt_br|ru_ru|sv_se|tr_tr|uk_ua|zh_cn|zh_tw)\.json'), re.compile(r'data/tconstruct/horizon/(?:item|block|menu|entity_type|mob_effect)/[a-z0-9_]+\.json'),
    # Machine-copied original data (tools/import_original_data.py): decoded by the production loaders in tests.
    re.compile(r'data/tconstruct/tinkering/(?:tool_definitions|modifiers|station_layouts|mob_equipment|materials/(?:definition|stats|traits))/[a-z0-9_]+\.json'),
    re.compile(r'data/tconstruct/tinkering/tags/(?:materials|modifiers|damage_types)(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
    re.compile(r'data/tconstruct/tinkering/enchantments_to_modifiers\.json'),
    re.compile(r'data/tconstruct/tags/(?:item|block|entity_type|fluid|enchantment|block_entity_type|potion|menu)(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
    # Client assets imported from the original (MIT, assets/tconstruct/license): item definitions, models, blockstates, textures, equipment assets.
    re.compile(r'assets/tconstruct/license'), re.compile(r'assets/tconstruct/(?:items|blockstates|models|textures|equipment)(?:/[a-z0-9_]+)*/[a-z0-9_]+\.(?:json|png|png\.mcmeta)'),
    # The original's sound definitions and recordings (tools/import_original_data.py copy_sounds).
    re.compile(r'assets/tconstruct/sounds\.json'), re.compile(r'assets/tconstruct/sounds/[a-z0-9_]+\.ogg'),
    # The fluid atlas sources (tools/import_original_data.py): the game's unified atlas (screen draws and
    # native block-entity draws) packs the walked texture folders and the base game's atlas definitions,
    # which name only block/ and entity/conduit/; textures/fluid/ is in neither, so no fluid sprite would
    # ever be stitched and every tank, basin and channel face naming one would be skipped. A content mod's
    # assets/minecraft/atlases/*.json is not copied by the host's builder: its sources are appended to the
    # base definition of the same name, so this one file adds the fluid directory to the blocks atlas.
    re.compile(r'assets/minecraft/atlases/blocks\.json'),
    # The books' Mantle data (tools/import_original_data.py): index, appearance, section files, per-language pages and images,
    # walked by books.rs through the client resources; the two .nbt files are the structure templates of the book images folder.
    re.compile(r'assets/tconstruct/book(?:/[a-z0-9_]+)*/[a-z0-9_]+\.(?:json|lang|png|nbt)'),
    re.compile(r'data/tconstruct/recipes/tools/(?:materials|modifiers|parts/builder|building|recycling|severing)(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
    # The material swapping recipes of the station (swapping_recipes.rs): the tool swapping recipe of the
    # tables tree and the fixed and material-value swapping recipes of the tools armor trees.
    re.compile(r'data/tconstruct/recipes/tables/tool_material_swapping\.json'),
    # The tinker station's own table recipes (table_recipes.rs, damaging.rs): part swapping for tools,
    # throwing weapons and ammo, station repair, and the damaging recipes of the damaging folder.
    re.compile(r'data/tconstruct/recipes/tables/(?:ammo_part_swapping|throwing_axe_part_swapping|tinker_station_part_swapping|tinker_station_repair)\.json'),
    re.compile(r'data/tconstruct/recipes/tables/tinker_station_damaging/[a-z0-9_]+\.json'),
    re.compile(r'data/tconstruct/recipes/tools/armor(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
    # The material render infos (the original's assets/tconstruct/tinkering/materials, imported verbatim):
    # display color, fallbacks and sprite-generator coverage, decoded by material_render.rs through the
    # client resources; a variant's file lives one folder below its material.
    re.compile(r'assets/tconstruct/tinkering/materials(?:/[a-z0-9_]+)?/[a-z0-9_]+\.json'),
    # The part builder's damageable recycling recipes of the tables tree (the tools tree's are under recycling above).
    re.compile(r'data/tconstruct/recipes/tables/recycling(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
    # The smeltery domain's melting, alloy, fuel, casting and molding recipes and the fluid tags they name (tools/import_original_data.py),
    # kept in the mod's own folders (the host's own copy of the tconstruct tags is data/tconstruct/tags/fluid, matched below).
    re.compile(r'data/tconstruct/recipes/fluids(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
    re.compile(r'data/tconstruct/tinkering/fluid_tags/(?:tconstruct|forge|mantle)(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
    # The fluid effects the engine of fluid_effects.rs decodes (tools/import_original_data.py), in the mod's own folder.
    re.compile(r'data/tconstruct/tinkering/fluid_effects/[a-z0-9_]+\.json'),
    # The particle types' sprite sets (the original's assets/tconstruct/particles, converted by hand for
    # particles.rs): the attack sets name their slash frames under textures/particle (the client-assets
    # shape above) and carry the host's horizon:behaviour block; the slime and fluid types ship empty.
    re.compile(r'assets/tconstruct/particles/[a-z0-9_]+\.json'),
    # The modifier icon manager's overrides (tools/import_original_data.py): the icon and book cover
    # sprite per modifier, named for the screens that draw modifier buttons; the sprites themselves
    # live under textures/gui/modifiers (the client-assets shape above).
    re.compile(r'assets/tconstruct/tinkering/modifier_icons\.json'),
    # Damage types used by armor modules and the smeltery, and their additions to vanilla tags.
    re.compile(r'data/tconstruct/damage_type/[a-z0-9_]+\.json'), re.compile(r'data/minecraft/tags/damage_type/(?:avoids_guardian_thorns|bypasses_armor|bypasses_cooldown|bypasses_effects|bypasses_enchantments|is_explosion|is_fire|is_freezing|is_projectile|witch_resistant_to)\.json'),
    # Trim materials, imported as data only: the entries exist in the trim material registry, but no trim
    # rendering consumer exists yet (the trim modifier's rendering side is a separate modifier-side gap).
    re.compile(r'data/tconstruct/trim_material/[a-z0-9_]+\.json'),
    # World content imported from the original (tools/import_original_data.py): block loot tables, ore features with their biome modifier,
    # and the content's own entries in vanilla's block tags (mineable, needs_*_tool, ...).
    re.compile(r'data/tconstruct/loot_table/blocks/[a-z0-9_]+\.json'), re.compile(r'data/tconstruct/loot_table/entities/[a-z0-9_]+\.json'), re.compile(r'data/tconstruct/worldgen/(?:feature|placed_feature)/[a-z0-9_]+\.json'),
    # Global loot modifiers and the list that orders them (the list keeps its Forge-era folder).
    re.compile(r'data/tconstruct/loot_modifiers/(?:lustrous/)?[a-z0-9_]+\.json'), re.compile(r'data/forge/loot_modifiers/global_loot_modifiers\.json'),
    # Mantle loot injectors (tools/import_original_data.py): the entries merged into vanilla tables
    # at load, with their top-level conditions evaluated against the live config.
    re.compile(r'data/tconstruct/mantle/loot_injectors/[a-z0-9_]+\.json'),
    # Slime islands (tools/import_original_data.py): the island structures and their sets, the island biome tags the
    # structures name, and the binary NBT templates the repalleter produced (payloads are not JSON; shape only).
    # Each template ships twice: the original's plural `structures/` folder and, byte-identical, the singular
    # `structure/` folder — the host's content-mod builder forwards only `data/<ns>/structure/**.nbt` into the
    # worldgen loader's template folder, so without the singular copies the loader's template store holds
    # nothing for the islands and every island start is rejected as a missing or empty template.
    re.compile(r'data/tconstruct/worldgen/(?:structure|structure_set)/[a-z0-9_]+\.json'),
    re.compile(r'data/tconstruct/structures/islands/(?:blood|dirt|earth|ender|sky)/[a-z0-9_]+\.nbt'),
    re.compile(r'data/tconstruct/structure/islands/(?:blood|dirt|earth|ender|sky)/[a-z0-9_]+\.nbt'),
    re.compile(r'data/tconstruct/tags/worldgen/biome(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
    re.compile(r'data/tconstruct/neoforge/biome_modifier/[a-z0-9_]+\.json'), re.compile(r'data/minecraft/tags/(?:block|item|entity_type)(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
    re.compile(r'data/c/tags/(?:block|item)/(?:ores(?:/cobalt)?|storage_blocks(?:/raw_cobalt)?|ore_rates/singular|ores_in_ground/netherrack)\.json'),
    re.compile(r'data/c/tags/item/raw_materials(?:/cobalt)?\.json'),
    # The original's common fluid tag (its data/c/tags/fluids, imported by copy_hidden_fluid_tag):
    # the fluids recipe viewers hide, referenced by tconstruct's hide_in_creative_tanks fluid tag.
    # The port's tag kinds are singular, so it lands under data/c/tags/fluid.
    re.compile(r'data/c/tags/fluid/hidden_from_recipe_viewers\.json'),
    # Gameplay advancements and the functions their internal advancements reward, converted by the importer
    # (display icons in the host's form); the starting book's advancement reward table rides with them.
    re.compile(r'data/tconstruct/advancement/(?:foundry|internal|smeltery|tools|world)/[a-z0-9_]+\.json'),
    re.compile(r'data/tconstruct/advancement/recipes(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
    re.compile(r'data/tconstruct/function/grant_advancement/[a-z0-9_]+\.mcfunction'),
    re.compile(r'data/tconstruct/loot_table/gameplay/[a-z0-9_]+\.json'),
    # Crafting, smelting, blasting and stonecutting recipes of the smeltery blocks and the commons, converted to the game's own recipe format by the importer.
    re.compile(r'data/tconstruct/recipe(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
)
CRAFTING_RECIPE_TYPES = {'minecraft:crafting_shaped', 'minecraft:crafting_shapeless', 'minecraft:smelting', 'minecraft:blasting', 'minecraft:stonecutting'}
FLUID_RECIPE_TYPES = {
    'tconstruct:melting', 'tconstruct:ore_melting', 'tconstruct:damagable_melting', 'tconstruct:material_melting', 'tconstruct:melting_fuel',
    'tconstruct:entity_melting', 'tconstruct:alloy', 'tconstruct:material_fluid', 'tconstruct:molding_table', 'tconstruct:item_part_builder',
    'tconstruct:casting_table', 'tconstruct:casting_basin', 'tconstruct:retextured_casting_basin', 'tconstruct:table_duplication',
    'tconstruct:casting_table_potion', 'tconstruct:table_filling', 'tconstruct:basin_filling',
    'tconstruct:table_casting_material', 'tconstruct:basin_casting_material', 'tconstruct:table_casting_composite', 'tconstruct:basin_casting_composite',
    'tconstruct:basin_tool_casting', 'tconstruct:table_tool_casting', 'tconstruct:basin_casting_part_swapping', 'tconstruct:table_casting_part_swapping',
    'tconstruct:casting_table_tipping', 'tconstruct:casting_table_tipped_clearing',
}
LOOT_CONDITIONS = {'minecraft:survives_explosion', 'minecraft:all_of', 'minecraft:block_state_property', 'minecraft:any_of', 'minecraft:inverted', 'minecraft:table_bonus', 'minecraft:match_tool', 'minecraft:damage_source_properties', 'minecraft:entity_properties'}
LOOT_FUNCTIONS = {'minecraft:explosion_decay', 'minecraft:apply_bonus', 'minecraft:set_count', 'minecraft:copy_custom_data', 'minecraft:looting_enchant', 'minecraft:furnace_smelt', 'mantle:fill_retextured_block'}
LOOT_ENTRIES = {'minecraft:item', 'minecraft:alternatives'}
FEATURE_TYPES = {'minecraft:ore', 'minecraft:geode', 'minecraft:tree', 'tconstruct:slime_tree', 'tconstruct:slime_fungus'}
ISLAND_STRUCTURE_KEYS = {'type', 'biomes', 'step', 'spawn_overrides', 'terrain_adaptation', 'dimension_padding', 'placement', 'templates', 'trees', 'vines', 'grasses'}
BIOME_MODIFIER_KEYS = {'type', 'biomes', 'features', 'step'}
DAMAGE_TYPE_KEYS = {'message_id', 'scaling', 'exhaustion', 'effects', 'death_message_type'}
TRIM_MATERIAL_KEYS = {'asset_name', 'description', 'ingredient', 'item_model_index'}
ITEM_KEYS = {'template', 'components', 'crafting_remainder', 'max_stack_size'}
BLOCK_KEYS = {'template', 'properties', 'item', 'components', 'state_properties', 'state_light', 'default_state', 'shapes', 'collision_shapes'}
MENU_KEYS = {'slots', 'data_slots', 'results', 'links'}
ENTITY_TYPE_KEYS = {'template', 'item', 'damage'}
MOB_EFFECT_KEYS = {'category', 'color'}


def check_loot(item, name):
    """Block and entity loot tables use only reviewed condition, function and entry kinds (the shapes the importer converts)."""
    def condition(node):
        if node not in ('minecraft:tool/can_silk_touch', 'minecraft:tool/can_shear') and not (isinstance(node, dict) and node.get('type') in LOOT_CONDITIONS):
            raise ValueError(f'unreviewed loot condition in {name}')
        for term in node.get('terms', []) if isinstance(node, dict) else []:
            condition(term)
        if isinstance(node, dict) and isinstance(node.get('term'), (str, dict)):
            condition(node['term'])

    def walk(node):
        if 'condition' in node:
            condition(node['condition'])
        for function in node.get('modifier', []):
            if function.get('type') not in LOOT_FUNCTIONS:
                raise ValueError(f'unreviewed loot function in {name}')
        if 'type' in node and 'entries' not in node and 'pools' not in node and node['type'] not in LOOT_ENTRIES:
            raise ValueError(f'unreviewed loot entry in {name}')
        for key in ('pools', 'entries', 'children'):
            for child in node.get(key, []):
                walk(child)
    if not isinstance(item, dict) or item.get('type') not in ('minecraft:block', 'minecraft:entity') or not isinstance(item.get('pools'), list) or not isinstance(item.get('random_sequence'), str):
        raise ValueError(f'a loot table needs the block or entity type, pools and a random sequence: {name}')
    walk(item)


LOOT_MODIFIER_TYPES = {'mantle:add_entry', 'mantle:replace_item', 'tconstruct:modifier_hook'}
LOOT_MODIFIER_CONDITIONS = {'mantle:block_tag', 'mantle:tag_filled', 'tconstruct:has_modifier', 'tconstruct:has_chrysophilite', 'tconstruct:has_context_set', 'tconstruct:block_or_entity', 'forge:loot_table_id', 'tconstruct:config', 'minecraft:entity_properties'}
LOOT_MODIFIER_POST_CONDITIONS = {'mantle:inverted', 'mantle:contains_item'}
LOOT_MODIFIER_FUNCTIONS = {'minecraft:set_count', 'minecraft:looting_enchant', 'minecraft:explosion_decay', 'tconstruct:modifier_bonus', 'tconstruct:chrysophilite_bonus'}
LOOT_MODIFIER_ENTRIES = {'minecraft:item', 'mantle:tag_preference'}


def check_loot_modifier(item, name):
    """Global loot modifiers use only the reviewed condition, function and entry kinds the port's engine decodes."""
    if name.endswith('global_loot_modifiers.json'):
        if not isinstance(item, dict) or set(item) != {'entries', 'replace'} or item['replace'] is not False \
                or not isinstance(item['entries'], list) or not all(isinstance(entry, str) for entry in item['entries']):
            raise ValueError(f'a global loot modifier list needs entries and replace false: {name}')
        return
    if not isinstance(item, dict) or item.get('type') not in LOOT_MODIFIER_TYPES:
        raise ValueError(f'unreviewed loot modifier type in {name}')

    def conditions(node, allowed, key):
        if not isinstance(node, list):
            raise ValueError(f'loot modifier conditions must be a list in {name}')
        for entry in node:
            kind = entry.get(key) if isinstance(entry, dict) else None
            if kind not in allowed:
                raise ValueError(f'unreviewed loot modifier condition in {name}')
            if kind == 'mantle:inverted':
                conditions([entry.get('condition')], LOOT_MODIFIER_POST_CONDITIONS, 'type')

    conditions(item.get('conditions', []), LOOT_MODIFIER_CONDITIONS, 'condition')
    if item['type'] == 'mantle:add_entry':
        conditions(item.get('post_conditions', []), LOOT_MODIFIER_POST_CONDITIONS, 'type')
        if item.get('entry', {}).get('type') not in LOOT_MODIFIER_ENTRIES:
            raise ValueError(f'unreviewed loot modifier entry in {name}')
    for function in item.get('functions', []):
        if function.get('function') not in LOOT_MODIFIER_FUNCTIONS:
            raise ValueError(f'unreviewed loot modifier function in {name}')
ADVANCEMENT_TRIGGERS = {'minecraft:inventory_changed', 'minecraft:placed_block', 'minecraft:location', 'minecraft:tick', 'minecraft:player_interacted_with_entity', 'minecraft:recipe_unlocked', 'tconstruct:block_container_opened'}
ADVANCEMENT_KEYS = {'parent', 'display', 'criteria', 'requirements', 'rewards', 'sends_telemetry_event'}
TOOL_PREDICATE_TYPES = {'tconstruct:has_modifier', 'tconstruct:has_material', 'tconstruct:has_volatile_key', 'tconstruct:tool_action', 'tconstruct:stat_in_range', 'tconstruct:stat_in_set', 'mantle:and', 'mantle:or', 'mantle:set', 'mantle:tag', 'mantle:inverted'}


def check_tool_predicate(node, name):
    """Tool predicates use only the forms the advancement consumer decodes (the original's
    advancement data forms; anything else is rejected there too, never treated as true)."""
    if isinstance(node, str):
        if node != 'tconstruct:has_upgrades':
            raise ValueError(f'unreviewed tool predicate singleton in {name}')
        return
    if not isinstance(node, dict) or node.get('type') not in TOOL_PREDICATE_TYPES:
        raise ValueError(f'unreviewed tool predicate in {name}')
    kind = node['type']
    if kind in ('mantle:and', 'mantle:or'):
        predicates = node.get('predicates')
        if not isinstance(predicates, list) or not predicates:
            raise ValueError(f'a composite tool predicate needs predicates in {name}')
        for sub in predicates:
            check_tool_predicate(sub, name)
    elif kind == 'mantle:inverted':
        inner = dict(node)
        inner['type'] = node.get('inverted_type')
        check_tool_predicate(inner, name)
    elif kind == 'mantle:set':
        if not isinstance(node.get('items'), list) or not all(isinstance(entry, str) for entry in node['items']):
            raise ValueError(f'a set tool predicate needs item IDs in {name}')
    elif kind == 'mantle:tag':
        if not isinstance(node.get('tag'), str):
            raise ValueError(f'a tag tool predicate needs a tag ID in {name}')
    elif kind == 'tconstruct:has_modifier':
        modifier = node.get('modifier')
        if not isinstance(modifier, dict) or modifier.get('type') != 'tconstruct:single' or not isinstance(modifier.get('modifier'), str):
            raise ValueError(f'only single-modifier matches are reviewed in {name}')
        if node.get('check') not in ('upgrades', 'all'):
            raise ValueError(f'a modifier match needs its check in {name}')
        level = node.get('level')
        if level is not None and not isinstance(level, (int, float)) and not (isinstance(level, dict) and set(level) <= {'min', 'max'}):
            raise ValueError(f'a modifier level must be a number or a range in {name}')
    elif kind == 'tconstruct:has_material':
        if not isinstance(node.get('material'), str) or ('index' in node and not isinstance(node['index'], int)):
            raise ValueError(f'a material match needs a variant string and an integer index in {name}')
    elif kind == 'tconstruct:has_volatile_key':
        if not isinstance(node.get('key'), str):
            raise ValueError(f'a volatile key match needs a key in {name}')
    elif kind == 'tconstruct:tool_action':
        if not isinstance(node.get('action'), str):
            raise ValueError(f'a tool action match needs an action in {name}')
    elif kind == 'tconstruct:stat_in_range':
        if not isinstance(node.get('stat'), str) or any(key in node and not isinstance(node[key], (int, float)) for key in ('min', 'max')):
            raise ValueError(f'a stat range needs a stat and numeric bounds in {name}')
    elif kind == 'tconstruct:stat_in_set':
        if not isinstance(node.get('stat'), str) or not isinstance(node.get('values'), list):
            raise ValueError(f'a stat set needs a stat and values in {name}')


def check_item_predicate(node, name):
    if not isinstance(node, dict) or set(node) - {'item', 'items', 'tag', 'count', 'durability', 'enchantments', 'nbt', 'potion', 'type', 'predicate'}:
        raise ValueError(f'unreviewed item predicate in {name}')
    if 'type' in node:
        if node['type'] != 'tconstruct:tool_stack' or 'predicate' not in node:
            raise ValueError(f'only the tool_stack item predicate is reviewed in {name}')
        check_tool_predicate(node['predicate'], name)
    for key in ('item', 'tag', 'potion'):
        if key in node and not isinstance(node[key], str):
            raise ValueError(f'an item predicate field must be a string in {name}')
    if 'items' in node and not (isinstance(node['items'], list) and all(isinstance(entry, str) for entry in node['items'])):
        raise ValueError(f'an item predicate items field must be item IDs in {name}')


def check_entity_predicates(node, name):
    """Entity predicate condition lists, as location and interaction criteria carry them."""
    if not isinstance(node, list):
        raise ValueError(f'entity conditions must be a list in {name}')
    for entry in node:
        if not isinstance(entry, dict) or set(entry) - {'condition', 'entity', 'predicate'} or entry.get('condition') != 'minecraft:entity_properties':
            raise ValueError(f'only entity_properties conditions are reviewed in {name}')
        predicate = entry.get('predicate') or {}
        if not isinstance(predicate, dict) or set(predicate) - {'type', 'location', 'equipment', 'stepping_on'}:
            raise ValueError(f'unreviewed entity predicate in {name}')
        if 'type' in predicate and not isinstance(predicate['type'], str):
            raise ValueError(f'an entity type predicate must be a string in {name}')
        location = predicate.get('location')
        if location is not None and (not isinstance(location, dict) or set(location) - {'structure', 'biome', 'dimension', 'position', 'smokey', 'light', 'block', 'fluid'}):
            raise ValueError(f'unreviewed location predicate in {name}')
        equipment = predicate.get('equipment')
        if equipment is not None:
            if not isinstance(equipment, dict):
                raise ValueError(f'equipment predicates must be an object in {name}')
            for slot_predicate in equipment.values():
                check_item_predicate(slot_predicate, name)


def check_advancement(item, name):
    """Gameplay advancements use only the reviewed triggers, condition shapes and predicate
    forms: the ones the host's engine and the port's advancement consumers evaluate."""
    if not isinstance(item, dict) or set(item) - ADVANCEMENT_KEYS:
        raise ValueError(f'unreviewed advancement keys in {name}')
    criteria = item.get('criteria')
    if not isinstance(criteria, dict) or not criteria:
        raise ValueError(f'an advancement needs criteria: {name}')
    display = item.get('display')
    if display is not None:
        if not isinstance(display, dict) or set(display) - {'icon', 'title', 'description', 'frame', 'background', 'show_toast', 'announce_to_chat', 'hidden'}:
            raise ValueError(f'unreviewed display keys in {name}')
        icon = display.get('icon')
        if not isinstance(icon, dict) or set(icon) != {'id'} or not isinstance(icon['id'], str):
            raise ValueError(f'a display icon must be a single item ID in {name}')
    rewards = item.get('rewards')
    if rewards is not None and (not isinstance(rewards, dict) or set(rewards) - {'recipes', 'loot', 'experience', 'function'}):
        raise ValueError(f'unreviewed rewards in {name}')
    requirements = item.get('requirements')
    if requirements is not None and not (isinstance(requirements, list) and all(
            isinstance(group, list) and all(isinstance(criterion, str) and criterion in criteria for criterion in group)
            for group in requirements)):
        raise ValueError(f'requirements must name criteria in {name}')
    for criterion in criteria.values():
        if not isinstance(criterion, dict) or set(criterion) - {'trigger', 'conditions'}:
            raise ValueError(f'unreviewed criterion keys in {name}')
        trigger = criterion.get('trigger')
        if trigger not in ADVANCEMENT_TRIGGERS:
            raise ValueError(f'unreviewed advancement trigger in {name}')
        conditions = criterion.get('conditions') or {}
        if not isinstance(conditions, dict):
            raise ValueError(f'criterion conditions must be an object in {name}')
        if trigger == 'minecraft:inventory_changed':
            if set(conditions) - {'items'}:
                raise ValueError(f'unreviewed inventory_changed conditions in {name}')
            for entry in conditions.get('items', []):
                check_item_predicate(entry, name)
        elif trigger == 'tconstruct:block_container_opened':
            if set(conditions) != {'type'} or conditions['type'] not in ('tconstruct:smeltery', 'tconstruct:foundry'):
                raise ValueError(f'a container opened criterion names the smeltery or the foundry in {name}')
        elif trigger == 'minecraft:placed_block':
            if set(conditions) - {'item', 'location', 'player'}:
                raise ValueError(f'unreviewed placed_block conditions in {name}')
            location = conditions.get('location')
            if not isinstance(location, list) or not all(isinstance(entry, dict) and set(entry) <= {'block', 'condition', 'properties'} for entry in location):
                raise ValueError(f'a placed_block location must be block predicates in {name}')
            if 'item' in conditions:
                check_item_predicate(conditions['item'], name)
        elif trigger == 'minecraft:location':
            if set(conditions) - {'player', 'stepping_on'}:
                raise ValueError(f'unreviewed location conditions in {name}')
            check_entity_predicates(conditions.get('player'), name)
        elif trigger == 'minecraft:player_interacted_with_entity':
            if set(conditions) - {'entity', 'item', 'player'}:
                raise ValueError(f'unreviewed interaction conditions in {name}')
            check_entity_predicates(conditions.get('entity'), name)
            if 'item' in conditions:
                check_item_predicate(conditions['item'], name)
        elif trigger == 'minecraft:tick':
            if conditions:
                raise ValueError(f'a tick criterion carries no conditions here in {name}')


def check_reward_loot(item, name):
    """Advancement reward tables (gameplay) give plain items and nothing else."""
    if not isinstance(item, dict) or item.get('type') != 'minecraft:advancement_reward' or not isinstance(item.get('pools'), list):
        raise ValueError(f'a gameplay loot table must be an advancement reward table with pools: {name}')
    for pool in item['pools']:
        for entry in pool.get('entries', []):
            if entry.get('type') != 'minecraft:item' or not isinstance(entry.get('name'), str):
                raise ValueError(f'only plain item entries are reviewed in a gameplay loot table: {name}')


def plan(root):
    data = json.loads((root / 'docs/port-plan.json').read_text(encoding='utf-8'))
    if data.get('schema') != 1:
        raise ValueError('unsupported plan schema')
    tasks = data['tasks']
    ids = [t['id'] for t in tasks]
    if len(ids) != len(set(ids)):
        raise ValueError('duplicate task IDs')
    by_id = {t['id']: t for t in tasks}
    visiting, visited = set(), set()
    def visit(ident):
        if ident in visiting:
            raise ValueError('cyclic task dependencies')
        if ident in visited:
            return
        if ident not in by_id:
            raise ValueError(f'unknown prerequisite: {ident}')
        visiting.add(ident)
        for parent in by_id[ident]['depends_on']:
            visit(parent)
        visiting.remove(ident)
        visited.add(ident)
    active = 0
    for task in tasks:
        visit(task['id'])
        if task['status'] not in ('pending', 'in_progress', 'blocked', 'done'):
            raise ValueError('invalid task state')
        active += task['status'] == 'in_progress'
        if task['status'] == 'blocked' and not task.get('blocker'):
            raise ValueError('blocked task needs a concrete prerequisite')
        if task['status'] in ('done', 'in_progress'):
            if any(by_id[d]['status'] != 'done' for d in task['depends_on']):
                raise ValueError(f'{task["id"]}: prerequisite not complete')
        if task['status'] == 'done':
            if not task.get('evidence'):
                raise ValueError('completed task needs evidence')
            if task['kind'] == 'gameplay' and not any(e.get('stage') == 'game' and e.get('input_digest') and e.get('scenario') for e in task['evidence']):
                raise ValueError('gameplay completion needs input-bound game evidence')
    if active > 1:
        raise ValueError('only one active integration unit')
    return data


def next_task(root):
    data = plan(root)
    done = {t['id'] for t in data['tasks'] if t['status'] == 'done'}
    for state in ('in_progress', 'pending'):
        for task in data['tasks']:
            if task['status'] == state and set(task['depends_on']) <= done:
                return task
    return None


def checked_files(root):
    data = plan(root)
    base = root / BUNDLE
    if base.is_symlink() or not base.is_dir():
        raise ValueError('bundle must be a real directory')
    names = data['bundle_files']
    if len(names) != len(set(names)):
        raise ValueError('duplicate bundle input')
    files = {}
    for name in names:
        rel = Path(name)
        if rel.is_absolute() or '..' in rel.parts or '\\' in name or ':' in name:
            raise ValueError(f'unsafe bundle path: {name}')
        source = base / rel
        if any(p.is_symlink() for p in [source, *source.parents] if p != root.parent):
            raise ValueError(f'symlink bundle input: {name}')
        source.resolve().relative_to(base.resolve())
        files[name] = source.read_bytes()
    actual = {p.relative_to(base).as_posix() for p in base.rglob('*') if p.is_file()}
    if actual != set(names):
        raise ValueError(f'unreviewed/missing bundle files: {sorted(actual ^ set(names))}')
    manifest = tomllib.loads(files['horizon-mixin.toml'].decode())
    if set(manifest) != {'mod', 'compatibility', 'api', 'dependencies'}:
        raise ValueError('Tinkers may declare only mod, compatibility, api and dependencies; no injections/modules/state/bridges/crates')
    if manifest['mod']['id'] != 'tconstruct' or manifest['api'] != {'path': 'api'}:
        raise ValueError('unexpected product or API entry')
    if manifest['dependencies'] != {'pluto_lib': '^0.1'}:
        raise ValueError('Tinkers must depend only on the public PlutoLib bundle')
    pins = json.loads((root / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    if manifest['compatibility'] != {'horizon_revision': pins['horizon_revision']}:
        raise ValueError('Horizon pin differs')
    cargo = tomllib.loads(files['api/Cargo.toml'].decode())
    if cargo['package']['version'] != manifest['mod']['version']:
        raise ValueError('product versions differ')
    if set(cargo) - {'package', 'lib', 'dependencies'} or set(cargo['package']) - {'name', 'version', 'edition', 'publish'}:
        raise ValueError('unreviewed Cargo build/configuration surface')
    if cargo['lib'] != {'path': 'src/lib.rs'} or cargo['package'].get('publish') is not False:
        raise ValueError('unexpected crate entry/release permission')
    loader_dependency = {'git': 'https://github.com/horizon-rust/horizon',
                         'rev': pins['horizon_revision'], 'default-features': False}
    if cargo.get('dependencies', {}).get('horizon-client') != loader_dependency:
        raise ValueError('required loader metadata must use the official pinned Horizon with defaults disabled')
    for name, value in cargo.get('dependencies', {}).items():
        if name == 'horizon-client':
            continue  # Owner-approved metadata only; Rust host references remain forbidden below.
        if name not in data['cargo_dependencies'] or not isinstance(value, str) or name.startswith(('horizon', 'tconstruct')):
            raise ValueError(f'forbidden/unreviewed Cargo dependency: {name}')
    forbidden = re.compile(r'\b(?:horizon(?:_[A-Za-z0-9_]+)?|horizon_mixins|include|include_str|include_bytes|env|option_env)!?\s*(?:::|!|\()|\b(?:legacy|__horizon_mixins_)\b|#\s*\[\s*path\s*=|\bextern\s+crate|\bunsafe\b|\bstd\s*::\s*(?:process|fs|net)\b')
    for name, payload in files.items():
        if name.endswith('.rs') and forbidden.search(payload.decode('utf-8')):
            raise ValueError(f'host/archive/dynamic-source boundary in {name}')
    # Other native manifests/build scripts must not hide outside the packaged tree.
    for p in (root / 'tconstruct').rglob('*'):
        if p.is_symlink():
            raise ValueError(f'symlink in active product: {p}')
        if p.is_file() and p.name in ('horizon-mixin.toml', 'Cargo.toml', 'build.rs') and p not in {base / n for n in names}:
            raise ValueError(f'unreviewed build entry: {p}')
    digest = hashlib.sha256()
    for name, payload in sorted(files.items()):
        digest.update(name.encode() + b'\0' + payload + b'\0')
    return files, manifest, pins, digest.hexdigest()


def checked_content(root, version):
    """The clean Horizon content mod (.hmod): manifest plus declaratively reviewed data.

    Registration is data-only: Horizon has no code path for content mods and PlutoLib
    exposes no registration API, so every declaration must be a plain vanilla-template item.
    """
    base = root / CONTENT
    if base.is_symlink() or not base.is_dir():
        raise ValueError('content root must be a real directory')
    files = {}
    for path in sorted(p for p in base.rglob('*') if p.is_file()):
        if any(q.is_symlink() for q in [path, *path.parents] if q != root.parent):
            raise ValueError(f'symlink content input: {path}')
        name = path.relative_to(base).as_posix()
        if not any(shape.fullmatch(name) for shape in CONTENT_PATHS):
            raise ValueError(f'unreviewed content file: {name}')
        files[name] = path.read_bytes()
    if 'horizon-mod.toml' not in files:
        raise ValueError('content manifest missing')
    expected = {'id': 'tconstruct', 'name': "Tinkers' Construct content", 'version': version,
                'api': '0.2', 'side': 'both', 'capabilities': ['registries'], 'namespaces': ['forge']}
    if tomllib.loads(files['horizon-mod.toml'].decode()) != expected:
        raise ValueError('content manifest differs from the reviewed one (version must match the native product)')
    for name, payload in files.items():
        if name.endswith('.json'):
            item = json.loads(payload.decode())
            if name == 'data/tconstruct/tinkering/enchantments_to_modifiers.json':
                # The enchantment-to-modifier map: enchantment (or #tag) IDs to modifier IDs.
                if not isinstance(item, dict) or not all(isinstance(key, str) and isinstance(value, str) for key, value in item.items()):
                    raise ValueError(f'the enchantment map must map ID strings to ID strings: {name}')
                continue
            if '/tinkering/' in name:
                if not isinstance(item, dict):
                    raise ValueError(f'tinkering data must be a JSON object: {name}')
                continue
            if '/recipes/fluids/' in name:
                # The smeltery domain's recipe types (see import_original_data.FLUID_RECIPE_TYPES), alone or wrapped in forge:conditional.
                wrapped = [entry.get('recipe', {}).get('type') for entry in item.get('recipes', [])] if isinstance(item, dict) and item.get('type') == 'forge:conditional' else [item.get('type') if isinstance(item, dict) else None]
                if not wrapped or any(kind not in FLUID_RECIPE_TYPES for kind in wrapped):
                    raise ValueError(f'only the reviewed smeltery recipe types are allowed here: {name}')
                continue
            # Advancements first: the recipe unlock advancements live under advancement/recipes/,
            # whose paths also contain '/recipes/'.
            if '/advancement/' in name:
                check_advancement(item, name)
                continue
            if '/recipes/' in name:
                if '/recipes/tools/modifiers/' in name:
                    reviewed = ('tconstruct:modifier', 'tconstruct:incremental_modifier', 'tconstruct:swappable_modifier', 'tconstruct:modifier_salvage', 'tconstruct:remove_modifier', 'tconstruct:multilevel_modifier', 'tconstruct:multilevel_incremental_modifier', 'tconstruct:overslime_modifier', 'tconstruct:enchantment_converting', 'tconstruct:armor_dyeing_modifier', 'tconstruct:modifier_repair', 'tconstruct:extract_modifier', 'tconstruct:modifier_set_worktable', 'tconstruct:modifier_sorting', 'tconstruct:toggle_interaction', 'tconstruct:banner_modifier', 'tconstruct:armor_trim_modifier')
                elif '/recipes/tools/parts/builder/' in name:
                    # part_builder_recipes.rs reads both kinds from this folder: the part recipes
                    # and the item part recipes (elytra_copying, the building folder's one).
                    reviewed = ('tconstruct:part_builder', 'tconstruct:item_part_builder')
                elif '/recipes/tools/severing/' in name:
                    # severing.rs loads all six kinds: the plain one and the five whose output reads the mob's own state.
                    reviewed = ('tconstruct:severing', 'tconstruct:ageable_severing', 'tconstruct:mooshroom_demushrooming', 'tconstruct:player_beheading', 'tconstruct:sheep_shearing', 'tconstruct:snow_golem_beheading')
                elif '/recipes/tools/building/' in name:
                    # table_recipes.rs reads both kinds from this folder: the tool building recipes
                    # and the tipped transform that turns a vanilla arrow into a tipped one.
                    reviewed = ('tconstruct:tool_building', 'tconstruct:tipped_tool_transform')
                elif '/recipes/tools/recycling/' in name:
                    reviewed = ('tconstruct:part_builder_tool_recycling', 'tconstruct:part_builder_recycling')
                elif '/recipes/tables/recycling/' in name:
                    reviewed = ('tconstruct:part_builder_recycling',)
                elif '/recipes/tables/' in name:
                    reviewed = ('tconstruct:tool_material_swapping', 'tconstruct:tinker_station_part_swapping', 'tconstruct:tinker_station_repair', 'tconstruct:tinker_station_damaging')
                elif '/recipes/tools/armor/' in name:
                    reviewed = ('tconstruct:fixed_material_swapping', 'tconstruct:material_value_swapping')
                else:
                    reviewed = ('tconstruct:material',)
                if not isinstance(item, dict) or item.get('type') not in reviewed:
                    raise ValueError(f'only {" or ".join(reviewed)} recipes are reviewed here: {name}')
                continue
            if name.startswith('assets/tconstruct/book/'):
                # Mantle book data: a book's index and its section files are page lists, its pages and appearance objects.
                if not isinstance(item, (dict, list)):
                    raise ValueError(f'book data must be a JSON object or a page list: {name}')
                continue
            if name.startswith('assets/tconstruct/') and not name.startswith('assets/tconstruct/lang/'):
                if not isinstance(item, dict):
                    raise ValueError(f'a client asset must be a JSON object: {name}')
                continue
            if name.startswith('assets/tconstruct/lang/'):
                # A language file: a flat object of translation keys to plain strings.
                if not isinstance(item, dict) or not all(isinstance(key, str) and isinstance(value, str) for key, value in item.items()):
                    raise ValueError(f'a language file must map keys to strings: {name}')
                continue
            if name == 'assets/minecraft/atlases/blocks.json':
                # The fluid atlas sources (see CONTENT_PATHS): the host's builder appends a mod's atlas
                # sources to the base definition of the same name, so the only reviewed shape is an object
                # whose sources are directory sources naming a texture folder and its sprite prefix.
                if not isinstance(item, dict) or set(item) != {'sources'} or not isinstance(item['sources'], list) or not item['sources'] or any(
                        not isinstance(source, dict) or set(source) != {'type', 'source', 'prefix'}
                        or source['type'] != 'minecraft:directory' or not isinstance(source['source'], str) or not isinstance(source['prefix'], str)
                        for source in item['sources']):
                    raise ValueError(f'an atlas definition must be an object with a sources list of directory sources: {name}')
                continue
            if '/loot_table/' in name:
                if '/gameplay/' in name:
                    check_reward_loot(item, name)
                else:
                    check_loot(item, name)
                continue
            if '/loot_modifiers/' in name:
                check_loot_modifier(item, name)
                continue
            if name.startswith('data/tconstruct/recipe/'):
                if not isinstance(item, dict) or item.get('type') not in CRAFTING_RECIPE_TYPES or not isinstance(item.get('result'), dict) or not str(item['result'].get('id', '')).count(':') == 1:
                    raise ValueError(f'only reviewed crafting, smelting, blasting and stonecutting recipes are allowed here: {name}')
                continue
            if '/worldgen/' in name:
                if not isinstance(item, dict):
                    raise ValueError(f'worldgen data must be a JSON object: {name}')
                if '/tags/' in name:
                    if set(item) - {'values', 'replace', 'remove'} or not isinstance(item.get('values'), list):
                        raise ValueError(f'a worldgen biome tag must be an object with a values list: {name}')
                    continue
                if '/worldgen/feature/' in name and item.get('type') not in FEATURE_TYPES:
                    raise ValueError(f'only {sorted(FEATURE_TYPES)} features are reviewed here: {name}')
                if '/worldgen/placed_feature/' in name and not (isinstance(item.get('feature'), str) and isinstance(item.get('placement'), list)):
                    raise ValueError(f'a placed feature needs a feature and a placement list: {name}')
                if '/worldgen/structure_set/' in name:
                    placement = item.get('placement')
                    if not isinstance(item.get('structures'), list) or not isinstance(placement, dict) or placement.get('type') != 'minecraft:random_spread':
                        raise ValueError(f'only random-spread structure sets are reviewed here: {name}')
                if '/worldgen/structure/' in name:
                    weighted = ('templates', 'trees', 'grasses')
                    entries_ok = all(isinstance(entry, dict) and isinstance(entry.get('data'), str) and isinstance(entry.get('weight'), int) and entry['weight'] > 0
                                       for key in weighted for entry in item.get(key, []))
                    if item.get('type') != 'tconstruct:island' or set(item) - ISLAND_STRUCTURE_KEYS or not entries_ok or not all(isinstance(item.get(key), list) for key in weighted):
                        raise ValueError(f'only the reviewed island structures are allowed here: {name}')
                continue
            if '/biome_modifier/' in name:
                if isinstance(item, dict) and item.get('type') == 'neoforge:add_spawns':
                    spawners = item.get('spawners')
                    if set(item) != {'type', 'biomes', 'spawners'} or not isinstance(item.get('biomes'), str) or not isinstance(spawners, list) or not spawners or any(
                            not isinstance(spawner, dict) or set(spawner) != {'type', 'weight', 'minCount', 'maxCount'}
                            or not isinstance(spawner['type'], str) or spawner['type'].count(':') != 1
                            or not isinstance(spawner['weight'], int) or spawner['weight'] < 1
                            or not isinstance(spawner['minCount'], int) or spawner['minCount'] < 0
                            or not isinstance(spawner['maxCount'], int) or spawner['maxCount'] < spawner['minCount'] for spawner in spawners):
                        raise ValueError(f'an add_spawns biome modifier names a biome set and reviewed spawner rows: {name}')
                    continue
                if not isinstance(item, dict) or set(item) != BIOME_MODIFIER_KEYS or item['type'] != 'neoforge:add_features' or not isinstance(item['step'], str):
                    raise ValueError(f'only neoforge:add_features biome modifiers are reviewed here: {name}')
                continue
            if name.startswith('data/minecraft/tags/'):
                # A mod's additions to vanilla's tags name only its own entries.
                entries = item.get('values', []) if isinstance(item, dict) else []
                if any(not str(entry if isinstance(entry, str) else entry.get('id', '')).lstrip('#').startswith('tconstruct:') for entry in entries):
                    raise ValueError(f'a vanilla tag addition may only name tconstruct entries: {name}')
            if '/tags/' in name:
                if not isinstance(item, dict) or set(item) - {'values', 'replace', 'remove'} or not isinstance(item.get('values'), list):
                    raise ValueError(f'tag must be an object with a values list: {name}')
                continue
            if name.startswith('data/tconstruct/damage_type/'):
                if not isinstance(item, dict) or set(item) - DAMAGE_TYPE_KEYS or not isinstance(item.get('message_id'), str) or item.get('scaling') not in ('never', 'always', 'when_caused_by_living_non_player') or not isinstance(item.get('exhaustion'), (int, float)):
                    raise ValueError(f'damage type needs a message id, a scaling and an exhaustion, and only reviewed keys: {name}')
                continue
            if name.startswith('data/tconstruct/trim_material/'):
                if not isinstance(item, dict) or set(item) != TRIM_MATERIAL_KEYS or not isinstance(item.get('asset_name'), str) or not isinstance(item.get('description'), dict) or not isinstance(item.get('ingredient'), str) or not isinstance(item.get('item_model_index'), (int, float)):
                    raise ValueError(f'trim material needs an asset name, a description, an ingredient and an item model index, and only reviewed keys: {name}')
                continue
            if '/menu/' in name:
                if not isinstance(item, dict) or set(item) - MENU_KEYS or not isinstance(item.get('slots'), int) or not 1 <= item['slots'] <= 256:
                    raise ValueError(f'menu declaration needs a slot count and only reviewed keys: {name}')
                if not all(isinstance(slot, int) and 0 <= slot < item['slots'] for slot in item.get('results', [])):
                    raise ValueError(f'menu result slots must exist: {name}')
                links = item.get('links')
                if links is not None and (not isinstance(links, dict) or set(links) - {'from', 'count'} or not isinstance(links.get('from'), int) or not isinstance(links.get('count'), int) or not 0 <= links['from'] or not 1 <= links['count'] or links['from'] + links['count'] > item['slots']):
                    raise ValueError(f'menu links must name a slot range with an integer from and count: {name}')
                continue
            if '/entity_type/' in name:
                if not isinstance(item, dict) or set(item) - ENTITY_TYPE_KEYS or not str(item.get('template', '')).startswith('minecraft:') or not isinstance(item.get('item'), str) or not isinstance(item.get('damage'), (int, float)):
                    raise ValueError(f'entity type declaration must copy a vanilla template with an item and a damage, and only reviewed keys: {name}')
                continue
            if '/mob_effect/' in name:
                if not isinstance(item, dict) or set(item) - MOB_EFFECT_KEYS or item.get('category') not in ('beneficial', 'harmful', 'neutral') or not isinstance(item.get('color'), int):
                    raise ValueError(f'mob effect declaration needs a category and a color, and only reviewed keys: {name}')
                continue
            keys = BLOCK_KEYS if '/block/' in name else ITEM_KEYS
            if not isinstance(item, dict) or set(item) - keys or not str(item.get('template', '')).startswith('minecraft:'):
                raise ValueError(f'declaration must copy a vanilla template with reviewed keys: {name}')
            if not all(isinstance(key, str) for key in item.get('components', {})):
                raise ValueError(f'invalid components in {name}')
            if '/block/' in name and not isinstance(item.get('properties', {}), dict):
                raise ValueError(f'block properties must be an object: {name}')
            if '/block/' in name:
                declared = item.get('state_properties', {})
                if not isinstance(declared, dict) or any(not isinstance(spec, dict) or spec.get('type') not in ('boolean', 'enum') for spec in declared.values()):
                    raise ValueError(f'block state properties must be booleans or enums: {name}')
                light = item.get('state_light')
                if light is not None and (not isinstance(light, dict) or light.get('property') not in declared or not isinstance(light.get('values'), dict)):
                    raise ValueError(f'state_light must select by a declared property: {name}')
    digest = hashlib.sha256()
    for name, payload in sorted(files.items()):
        digest.update(name.encode() + b'\x00' + payload + b'\x00')
    return files, digest.hexdigest()


def package_content(root, output):
    files, manifest, _, _ = checked_files(root)
    content, digest = checked_content(root, manifest['mod']['version'])
    output = output.resolve()
    output.relative_to((root / '.local').resolve())
    if output.exists():
        raise ValueError('output exists; choose a new package name')
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'x') as archive:
        for name, payload in sorted(content.items()):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, payload)
    with zipfile.ZipFile(output) as archive:
        if archive.testzip() or set(archive.namelist()) != set(content) or any(archive.read(n) != b for n, b in content.items()):
            raise ValueError('content package read-back differs')
    return digest


def release_preflight(root):
    """Reject unfinished clean ports; passing is not permission to publish.

    Artifact identity, native/runtime evidence review and tag validation remain
    separate gates. Development packaging deliberately does not call this.
    """
    data = plan(root)
    if not any(task['id'] == 'R5' and task['kind'] == 'gameplay' for task in data['tasks']):
        raise ValueError('release requires the whole-product R5 gameplay review')
    unfinished = [task['id'] for task in data['tasks'] if task['status'] != 'done']
    if unfinished:
        raise ValueError('clean port is not release-ready; unfinished tasks: ' + ', '.join(unfinished))
    return checked_files(root)[3]


def package_entries(root):
    """Return the exact checked archive payload, shared by packaging and validation."""
    files, manifest, pins, digest = checked_files(root)
    entries = {'horizon-native.json': (json.dumps({'schema': 1, 'id': 'tconstruct_native',
        'version': manifest['mod']['version'], 'game': '26.3', 'horizon': pins['horizon_revision'],
        'mixins': pins['mixins_revision']}, indent=2) + '\n').encode()}
    for name, payload in files.items():
        if name == 'horizon-mixin.toml':
            payload = payload.replace(b'id = "tconstruct"', b'id = "tconstruct_native"', 1)
        entries['mixins/tconstruct_native/' + name] = payload
    if len(entries) > 512 or sum(map(len, entries.values())) > 64 * 1024 * 1024:
        raise ValueError('official native archive limits exceeded')
    return entries, digest


def package(root, output):
    entries, digest = package_entries(root)
    output = output.resolve()
    # Packages are disposable outputs, never source/archive overwrites.
    output.relative_to((root / '.local').resolve())
    if output.exists():
        raise ValueError('output exists; choose a new package name')
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'x') as archive:
        for name, payload in sorted(entries.items()):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, payload)
    with zipfile.ZipFile(output) as archive:
        if archive.testzip() or set(archive.namelist()) != set(entries) or any(archive.read(n) != b for n, b in entries.items()):
            raise ValueError('package read-back differs')
    return digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['next', 'check', 'package', 'package-content', 'release-check'])
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    try:
        if args.action == 'next':
            task = next_task(ROOT)
            if task is None:
                print('BLOCKED: no ready work; report concrete prerequisites once.')
                return 2
            print(json.dumps(task, ensure_ascii=False, indent=2))
        elif args.action == 'release-check':
            print(f'PASS: release prerequisites; product sha256={release_preflight(ROOT)}')
            print('Artifact/tag validation, native/runtime evidence review and publishing authorization are still required.')
        elif args.action == 'check':
            files, manifest, _, digest = checked_files(ROOT)
            print(f'PASS: {len(files)} explicit clean-product inputs; zero injection declarations; sha256={digest}')
            content, content_digest = checked_content(ROOT, manifest['mod']['version'])
            print(f'PASS: {len(content)} content-mod inputs (declarations only); sha256={content_digest}')
            print('Native linkage, gameplay parity and full retained CI: NOT VERIFIED by this check.')
        elif args.action == 'package-content':
            if args.output is None:
                raise ValueError('--output is required')
            print(f'PACKAGED content mod (not loaded/game-verified): {package_content(ROOT, args.output)}')
        else:
            if args.output is None:
                raise ValueError('--output is required')
            print(f'PACKAGED (not compiled/game-verified): {package(ROOT, args.output)}')
        return 0
    except (ValueError, KeyError, OSError, TypeError) as error:
        print(f'FAIL: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
