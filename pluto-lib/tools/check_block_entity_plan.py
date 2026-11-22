#!/usr/bin/env python3
"""Fail closed until native attachment factory, client-state and cache boundaries are reviewed.

Synthetic fixtures test policy, never native attachment or a real plan review.
Existing exact owner/source/contract checks are retained, not relaxed.
"""
import importlib.util
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('block_entity_contract', ROOT / 'tools/check_structure_plan.py')
CONTRACT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CONTRACT)
TARGET = "horizon_client::game::native_hooks::Game<'_>::new_block_entity"
STATE_TARGET = 'horizon_client_render::chunk::block_entity::ModBlockEntity'
CACHE_STATE_TARGET = 'horizon_client::game::native_block_entity_draw::KeptEntry'
OWNER_FILE = 'crates/horizon-client-render/src/chunk/block_entity.rs'
CACHE_OWNER_FILE = 'crates/horizon-client/src/game/native_block_entity_draw.rs'
OWNERS = {
    'horizon_client::game::combat::CombatTracker': ('pluto-living-drop-state', 'crates/horizon-client/src/game/combat.rs'),
    STATE_TARGET: ('pluto-block-entity-client-state', OWNER_FILE),
    CACHE_STATE_TARGET: ('pluto-block-entity-cache-generation', CACHE_OWNER_FILE),
    'horizon_client_lib::mobs::nav::Navigation': ('pluto-entity-spawn-state', 'crates/horizon-client/src/mobs/nav.rs'),
}
MASK_TARGET = 'horizon_client_lib::mobs::enderman::stared_at'
TRADE_TARGET = "horizon_client::game::trading::Game<'_>::offers_from_set"
TRADE_FILES = {'crates/horizon-client/src/merchant.rs', 'crates/horizon-client/src/game/trading.rs'}
RELOAD_TARGET = "horizon_client::game::native_reload_listeners::Game<'_>::native_reload_listeners"
RELOAD_APPLY_TARGET = "horizon_client::game::native_hooks::Game<'_>::native_server_reload_listener_apply"
RELOAD_FILES = {'crates/horizon-client-core/src/registries.rs', 'crates/horizon-client/src/game/chat.rs',
                'crates/horizon-client/src/game/native_reload_listeners.rs',
                # Public host re-exports transport the exact adapted registry /
                # world types to trait packages; renamed foreign modules here
                # must not escape the same attachment review.
                'crates/horizon-client/src/client_lib.rs'}
SPAWN_TARGETS = {"horizon_client::game::creatures::Game<'_>::spawn_mob_internal",
                 "horizon_client::game::creatures::Game<'_>::convert_to",
                 "horizon_client::game::creatures::Game<'_>::finalize_converted",
                 "horizon_client::game::natural_spawning::Game<'_>::finalize_spawned",
                 "horizon_client::game::natural_spawning::Game<'_>::add_spawned",
                 "horizon_client::game::spawner::Game<'_>::plan_spawner_mob",
                 "horizon_client::game::spawner::Game<'_>::add_planned",
                 "horizon_client::game::raids::Game<'_>::spawn_event_mob",
                 "horizon_client::game::persist::Game<'_>::load_mob"}
SPAWN_FILES = {'crates/horizon-client/src/game/creatures.rs', 'crates/horizon-client/src/game/natural_spawning.rs',
               'crates/horizon-client/src/game/spawner.rs', 'crates/horizon-client/src/game/raids.rs',
               'crates/horizon-client/src/game/persist.rs', 'crates/horizon-client/src/mobs.rs',
               'crates/horizon-client/src/mobs/nav.rs'}
HOOK_TARGETS = SPAWN_TARGETS | {TARGET, MASK_TARGET, TRADE_TARGET, RELOAD_TARGET, RELOAD_APPLY_TARGET,
    "horizon_client::game::creatures::Game<'_>::spawn_mob_reason",
    "horizon_client::game::creatures::Game<'_>::spawn_mob_reason_yaw",
    "horizon_client::game::native_hooks::GameLevel<'_>::native_update_shape",
    'horizon_items::items::Items::load_with',
    'horizon_client::item_render::ItemRenderer::resolve_with_rule',
    'horizon_client_lib::item_render::ItemRenderer::resolve_with_rule',
    "horizon_client::game::Game<'_>::particle_sprite",
    "horizon_client::game::block_particles::Game<'_>::particle_sprite",
    "horizon_client::game::native_client_particles::Game<'_>::native_add_particle",
    "horizon_client::game::native_hooks::Game<'_>::native_entity_join_level",
    "horizon_client::game::commands::data::Game<'_>::apply_entity_nbt",
    'horizon_client_lib::mobs::finalize::set_base', 'horizon_client::mobs::finalize::set_base',
    'horizon_client_lib::mobs::finalize::compute', 'horizon_client::mobs::finalize::compute',
    'horizon_client_lib::mobs::finalize::has_modifier', 'horizon_client::mobs::finalize::has_modifier',
    'horizon_client_lib::mobs::finalize::modifier_amount', 'horizon_client::mobs::finalize::modifier_amount',
    'horizon_client_lib::mobs::finalize::remove_modifier', 'horizon_client::mobs::finalize::remove_modifier',
    'horizon_client_lib::mobs::living::Mob::cause_fall_damage_at',
    'horizon_client::mobs::living::Mob::cause_fall_damage_at',
    'horizon_client_lib::mobs::living::Mob::jump_power',
    'horizon_client::mobs::living::Mob::jump_power',
    'horizon_client_lib::mobs::cube::attack_damage',
    'horizon_client::mobs::cube::attack_damage',
    'horizon_client_lib::mobs::cube::landing_splash',
    'horizon_client::mobs::cube::landing_splash',
    "horizon_client::game::creatures::Game<'_>::handle_mob_events",
    "horizon_client::game::creatures::Game<'_>::drop_death_loot",
    "horizon_client::game::creatures::Game<'_>::tick_creature_entities",
    'horizon_client_lib::mobs::drops::roll',
    'horizon_client::mobs::drops::roll',
    'horizon_client::game::native_hooks::mob_kind',
    'horizon_client_render::chunk::block_entity::ModBlockEntity::revision',
    'horizon_client::game::native_block_entity_draw::KeptGeometry::get',
    "horizon_client::game::block_entity_anims::Game<'_>::tick_block_entity_anims",
    "horizon_client::game::remote::Game<'_>::enter_remote_dimension",
    "horizon_client::game::native_hooks::Game<'_>::native_client_level_unload",
    'horizon_assets::item_model::special::SpecialModel::decode',
    'horizon_assets::item_model::SpecialModel::decode',
    'horizon_client_lib::item_render::special::load_parts',
    'horizon_client_lib::item_render::special::extents',
    'horizon_client_lib::item_render::special::submit',
    'horizon_client::item_render::special::load_parts',
    'horizon_client::item_render::special::extents',
    'horizon_client::item_render::special::submit',
    'horizon_client::game::armor_stands::skull_kind',
    'horizon_client::mod_content::Plan::override_gameplay',
    "horizon_client_lib::mod_content::Plan<'_>::override_gameplay",
    'horizon_client::mod_content::resource_root',
    "horizon_client::game::machines::native_items::Game<'_>::native_container_slots",
    "horizon_client::mod_content::Overlay<'_>::write",
    'horizon_items::enchant::Enchantments::enchantable_of',
    "horizon_client::game::mounts::Game<'_>::shear_saddle"}


# Detect both historical re-export spellings and current owning-crate targets.
# Exact acceptance still comes from the pinned contract, not these aliases.
HOOK_TARGETS |= {target.replace('horizon_client::', 'horizon_client_lib::', 1)
                 for target in HOOK_TARGETS if target.startswith('horizon_client::')}
HOOK_TARGETS.add('horizon_client::mobs::nav::Navigation')
HOOK_TARGETS.add('horizon_client_lib::mobs::Mob')


def metadata():
    return json.loads((ROOT / 'tools/block-entity-boundaries.json').read_text(encoding='utf-8'))


def kept(plan):
    """Detect aliases/foreign owners too; the adapter cannot omit its guard."""
    slots = {('crates/horizon-client/src/game.rs', 'pluto_living_drops'),
             ('crates/horizon-client/src/game.rs', 'pluto_item_particles'),
             ('crates/horizon-client-render/src/mesher.rs', 'pluto_block_colors'),
             ('crates/horizon-client/src/entry.rs', 'pluto_block_entity_access'),
             ('crates/horizon-client-render/src/lib.rs', 'pluto_client_block_entity_state'),
             ('crates/horizon-client/src/game.rs', 'pluto_block_entity_cache'),
             ('crates/horizon-client/src/game.rs', 'pluto_client_block_entity_tick'),
             ('crates/horizon-client/src/game/remote.rs', 'pluto_client_block_entity_level'),
             ('crates/horizon-client/src/mod_content.rs', 'pluto_item_placement_export'),
             ('crates/horizon-client/src/mod_content.rs', 'pluto_block_settings_access'),
             ('crates/horizon-world/src/level/mod.rs', 'pluto_entity_type_access'),
             ('crates/horizon-client/src/entry.rs', 'pluto_entity_spawn_access'),
             ('crates/horizon-client/src/entry.rs', 'pluto_living_load_access'),
             ('crates/horizon-client/src/game.rs', 'pluto_entity_spawn_game')}
    files = {file for _, file in OWNERS.values()}
    return any(entry.get('module_id', entry.get('injection_id', entry.get('state_id', ''))).startswith(('pluto-block-entity-', 'pluto-block-settings-', 'pluto-skull-', 'pluto-item-placement-', 'pluto-entity-type-', 'pluto-entity-spawn-', 'pluto-entity-particle', 'pluto-living-drop', 'pluto-living-load-', 'pluto-living-entity-', 'pluto-living-attribute', 'pluto-head-', 'pluto-entity-kind'))
               or entry.get('module_id', entry.get('injection_id', '')).startswith(('pluto-item-behavior-', 'pluto-item-ender-mask', 'pluto-trade-', 'pluto-reload-'))
               or (entry.get('file'), entry.get('name')) == ('crates/horizon-world/src/level/mod.rs', 'pluto_item_behavior_access')
               or entry.get('file') in {'crates/horizon-client/src/mobs/finalize.rs', 'crates/horizon-assets/src/item_model.rs', 'crates/horizon-assets/src/item_model/special.rs', 'crates/horizon-client/src/item_render/special.rs', 'crates/horizon-client/src/game/armor_stands.rs', 'crates/horizon-client/src/mod_content.rs', 'crates/horizon-client/src/mobs/enderman.rs', 'crates/horizon-items/src/items.rs', 'crates/horizon-client/src/item_render.rs', 'crates/horizon-client/src/game/native_client_particles.rs', 'crates/horizon-client/src/mobs/cube.rs', 'crates/horizon-client/src/mobs/drops.rs'}
               or entry.get('file') in TRADE_FILES | RELOAD_FILES | SPAWN_FILES
               or entry.get('target') in HOOK_TARGETS | set(OWNERS) | {'horizon_client::mobs::Mob'}
               or entry.get('file') in files
               or any(c.get('file') in files | SPAWN_FILES for c in entry.get('constructors', []))
               or (entry.get('file'), entry.get('name')) in slots
               for section in ('modules', 'injections', 'states', 'bridges')
               for entry in plan.get(section, []))


def _node(node):
    return isinstance(node, list) and bool(node) and all(type(n) is int and n >= 0 for n in node)


def _fingerprint(value):
    return re.fullmatch('[0-9a-f]{64}', value or '')


def check(plan, workspace, meta=None, helpers=None, direct_receipt=None):
    meta = metadata() if meta is None else meta
    if meta.get('review_status') != 'reviewed':
        raise ValueError('Custom block-entity boundaries await the native attachment combined plan review; native build is blocked')
    transferred = set()
    if direct_receipt is not None or meta.get('direct_attachments'):
        if direct_receipt is None:
            raise ValueError('Reviewed direct attachments require their receipt')
        transferred = CONTRACT.check_direct_inputs(plan, workspace, meta, direct_receipt,
                                                   ROOT / 'mod' if helpers is None else helpers)
    if any(entry.get('target') in {'horizon_client::mobs::Mob', 'horizon_client_lib::mobs::Mob'} for entry in plan.get('states', [])):
        raise ValueError('Externally constructible Mob cannot own injected state; use the reviewed private Navigation boundary')
    mask = [i for i in plan.get('injections', []) if i.get('target') == MASK_TARGET]
    mask_modules = [m for m in plan.get('modules', []) if m.get('name') == 'pluto_item_behavior_access']
    if (mask or mask_modules) and (not any(i.get('target') == MASK_TARGET for i in meta['injections'])
            or not any(m.get('name') == 'pluto_item_behavior_access' for m in meta['modules'])):
        raise ValueError('Item ender-mask boundary has no reviewed native attachment source/site contract')
    trades = any(i.get('target') == TRADE_TARGET for i in plan.get('injections', [])) or any(
        m.get('file') in TRADE_FILES for m in plan.get('modules', []))
    if trades and (not any(i.get('target') == TRADE_TARGET for i in meta['injections'])
            or not {'pluto_trade_access', 'pluto_trade_pool'} <= {m.get('name') for m in meta['modules']}):
        raise ValueError('Trade pool boundary has no reviewed native attachment source/site contract')
    if any(i.get('target') == RELOAD_APPLY_TARGET for i in plan.get('injections', [])):
        raise ValueError('Reload apply-only hook cannot preserve the shared pack/resource snapshot')
    reloads = any(i.get('target') == RELOAD_TARGET for i in plan.get('injections', [])) or any(
        m.get('file') in RELOAD_FILES for m in plan.get('modules', []))
    if reloads and (not any(i.get('target') == RELOAD_TARGET for i in meta['injections'])
            or not {'pluto_reload_registry', 'pluto_reload_registry_source'} <= {m.get('name') for m in meta['modules']}):
        raise ValueError('Reload registry boundary has no reviewed native attachment source/site contract')
    spawns = any(i.get('target') in SPAWN_TARGETS or i.get('file') in SPAWN_FILES for i in plan.get('injections', []))
    if spawns and (not SPAWN_TARGETS <= {i.get('target') for i in meta['injections']}
            or not {'pluto_entity_spawn_access', 'pluto_entity_spawn_game'} <= {m.get('name') for m in meta['modules']}):
        raise ValueError('Complete mob spawn boundary has no reviewed native attachment source/site contract')
    for injection in meta['injections']:
        if injection['injection_id'] in transferred:
            continue
        if not _fingerprint(injection.get('function_fingerprint')):
            raise ValueError('Custom block-entity hook has no reviewed function fingerprint')
    states = meta['states']
    if len(states) != len(OWNERS) or {s['target'] for s in states} != set(OWNERS):
        raise ValueError('Custom block-entity state owner contracts are missing or changed')
    for required in states:
        expected_id, expected_file = OWNERS[required['target']]
        if (required.get('state_id'), required.get('file')) != (expected_id, expected_file):
            raise ValueError('Custom block-entity state owner location differs')
        if required.get('deserialize') is not False:
            raise ValueError('Custom block-entity client/cache state must not be deserialized')
        if required['state_id'] in transferred:
            # The exact reviewed full-file output replaces an AST constructor
            # proof only for this owner; all other state contracts stay intact.
            continue
        if not _fingerprint(required.get('fingerprint')) or not _node(required.get('node')):
            raise ValueError('Custom block-entity state has no reviewed owner fingerprint/node')
        constructors = required['constructors']
        if not isinstance(constructors, list) or len(constructors) != 1:
            raise ValueError('Custom block-entity state requires its exact reviewed constructor')
        for constructor in constructors:
            if (constructor.get('file') != expected_file or not _node(constructor.get('node'))
                    or type(constructor.get('expression')) is not int or constructor['expression'] < 0
                    or any(not _fingerprint(constructor.get(k)) for k in ('function_fingerprint', 'fingerprint'))):
                raise ValueError('Custom block-entity state has no reviewed constructor fingerprint/site')
        selected = [entry for entry in plan.get('states', [])
                    if entry.get('state_id') == required['state_id'] or entry.get('target') == required['target']]
        if len(selected) != 1:
            raise ValueError('Missing, duplicated or competing custom block-entity state')
        entry = selected[0]
        if entry.get('mod_id') != 'pluto_lib' or any(entry.get(k) != v for k, v in required.items()):
            raise ValueError('Custom block-entity state owner/initialization/constructor contract differs')
    CONTRACT.check(plan, workspace, meta, helpers, direct_receipt)
    files = {file for _, file in OWNERS.values()}
    state_ids = {s['state_id'] for s in states}
    injection_ids = {i['injection_id'] for i in meta['injections']}
    for entry in plan.get('states', []):
        if entry.get('file') in files or any(c.get('file') in files for c in entry.get('constructors', [])):
            if entry.get('state_id') not in state_ids:
                raise ValueError('Unreviewed state/constructor at custom block-entity boundary')
    for entry in plan.get('injections', []):
        if entry.get('file') in files and entry.get('injection_id') not in injection_ids:
            raise ValueError('Unreviewed injection at custom block-entity owner boundary')
    # A state helper may live alongside its owner (the Mob spawn-state module).
    # CONTRACT.check above still verifies its exact reviewed module/source/owner;
    # do not allow an arbitrary owner-file module or any unreviewed bridge.
    module_ids = {m['module_id'] for m in meta['modules']}
    for section in ('modules', 'bridges'):
        for entry in plan.get(section, []):
            if entry.get('file') in files:
                if section == 'modules' and entry.get('module_id') in module_ids:
                    continue
                raise ValueError('Unreviewed ' + section + ' at custom block-entity owner boundary')
