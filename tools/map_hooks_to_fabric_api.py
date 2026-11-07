#!/usr/bin/env python3
"""Map every TConstruct injection to the Fabric-API-like PlutoLib module that should replace it.

The table drives pluto-lib/docs/fabric-api-hook-map.md. Rules are keyed on the
host function the injection targets (and, where that is generic, the injection id).
Anything that matches no rule is listed as UNMAPPED so the design review sees it.

  map_hooks_to_fabric_api.py --out pluto-lib/docs/fabric-api-hook-map.md
"""
import argparse, collections, re, sys, tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# (module, Fabric API counterpart (Java names), regex on the host function name, regex on the injection id)
RULES = [
    ('worldgen', 'No Fabric counterpart (NeoForge-style structure hooks); `BiomeModifications` for features',
     '^(try_generate|use_saved_start)$', 'slime-island|slime-tree'),
    ('mining', 'Tool component rules (`Item.Settings.component(TOOL)`) + `PlayerBlockBreakEvents.BEFORE/AFTER/CANCELED`',
     '^(mining_speed|correct_for_drops|mine_block_with_held|tick_mining|destroy_progress|break_block_as_player|guest_destroy|native_destroy_block)$', None),
    ('screen-handler-api-v1', '`ScreenHandler`, `ScreenHandlerType` / `ExtendedScreenHandlerType`, `HandledScreens.register`, `Slot`',
     '^(menu_click|container_click|click_inventory|menu_ops|inventory_press|inventory_drag_move|inventory_mouse_release|slots|set_menu_slot|close_container|open_station|station_\\w+|remote_menu_ops|remote_station_\\w+|lan_station_\\w+|lan_open_container|lan_open_crafting|lan_crafting_\\w+|lan_table_valid|lan_send_table_content|remote_close_local|guest_container_closed|container_parts|slot_position|slot_view|screen_height|inventory_origin|quick_craft_accepts|clicked_outside|drag_view|tick_container_reach|open_native_screen|send_clicks|read|write|get|pre_remove|tick_craft_table|place_recipe|lan_place_recipe|tick_guests|lan_guest_packet|lan_station_rename)$', None),
    ('rendering-v1', '`HudRenderCallback`, `ItemTooltipCallback`, `BuiltinItemRendererRegistry`, `DrawContext` decorations, `ModelLoadingPlugin`',
     '^(draw_item_model|draw_item_decorations|draw_chest_screen|draw_inventory_slots|draw_slot_tooltip|stack_tooltip|rarity_color|held_name|draw_book|prepare_lod|insert|region|skips|entity_geometry|load_with)$', 'storage-(atlas|region|render)|connected-'),
    ('networking-api-v1', '`ServerPlayNetworking`, `ClientPlayNetworking`, `PlayerLookup.tracking`, `BlockEntityUpdateS2CPacket`',
     '^(handle_packet|track_entities|remote_block_entity_data|remote_synced|body_with|block_entity_type|remote_use_book|remote_start_held_use|remote_start_attack|remote_continue_attack|held_until_released|item_has_use|remote_block_reacts)$', None),
    ('block-entity (object-builder-api-v1 + data-attachment-api-v1)', '`FabricBlockEntityTypeBuilder`, `BlockEntity#readNbt/writeNbt`, `ServerBlockEntityEvents`, `AttachmentType`',
     '^(tick_block_entities|remove_entity|block_entity_id|decode_block_entity|record_saved|attach|set|remove|clear|set_raw|tick_level|process_level_events)$', None),
    ('block-behavior (object-builder-api-v1)', '`FabricBlockSettings`, `Block#getStateForNeighborUpdate/canPlaceAt/randomTick/onUse`, `BlockApiLookup`',
     '^(native_state_for_placement|native_can_survive|native_is_valid_bonemeal_target|native_is_bonemeal_success|native_perform_bonemeal|native_random_tick|native_update_shape|native_tick_block|apply_item_block_state|native_has_analog_output|native_analog_output|picked_stack|hopper_eject|hopper_suck|refuses)$', None),
    ('item-api-v1 + events-interaction-v0', '`FabricItem` (finishUsing, getMaxUseTime, getUseAction), `UseItemCallback`, `UseBlockCallback`, `UseEntityCallback`',
     '^(use_tool|use_hand|use_potion_item|use_on_cauldron|use_block_transformer|native_use_\\w+|native_living_entity_use_item_\\w+|trigger_using_item|tick_used_item|release_used_item|use_crossbow|charge_ticks|native_interact_entity|interact_entity|look_at_entity|lan_swing_back|predict_item_on_block|native_use_item_on|use_book|use_saved_start)$', None),
    ('entity-events-v1 + item-api-v1', '`ServerLivingEntityEvents.ALLOW_DAMAGE/AFTER_DEATH`, `ModifyItemAttributeModifiersCallback`, `CustomDamageHandler`, `AttackEntityCallback`',
     '^(attack_mob|weapon_hit|melee_damage|sweep_attack_position|sweep_damage|pvp_land|attack_damage|main_hand_attack_speed|weapon_wear|damage_held|hurt_armor|apply_enchantment_attributes|wearing_gold|add_projectile|projectile_hit|location_changed_effects|enter_guest|tick_local_hand|environment_tick)$', None),
    ('resource-loader-v0 + loot-api-v2 + recipe-api', '`ResourceManagerHelper`, reload listeners, `LootTableEvents.MODIFY`, `RecipeSerializer`, `ResourceConditions`',
     '^(native_data_pack_files|content_overlay|overlay_map|load_resources|ensure_datapacks|apply_pack_change|load|update_world|main|native_loot_function|assemble_if_matches|remainders)$', 'recipe-'),
    ('command-api-v2 + lifecycle-events-v1', '`CommandRegistrationCallback`, `ServerTickEvents`, `ServerLifecycleEvents`',
     '^(run_command|cmd_reload|tick|tick_advancements)$', None),
    ('transfer-api-v1', '`Storage<FluidVariant>` / `Storage<ItemVariant>`, `FluidStorage`, `ItemStorage`, `Transaction`',
     None, '.*(fluid|tank|faucet|channel|chute|basin)'),
    ('mod-specific (stays in Tinkers on top of the APIs above)', 'none: Tinkers book / station text input',
     '^(book_input|book_click|turn_page|station_text|station_key|station_typing|station_shortcut)$', None),
]


def classify(fn, ident):
    for module, fabric, fn_re, id_re in RULES:
        if fn_re and re.search(fn_re, fn):
            return module, fabric
        if id_re and re.search(id_re, ident):
            return module, fabric
    return None, None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--out', type=Path, required=True)
    a = ap.parse_args()
    manifest = tomllib.loads((ROOT / 'tconstruct/mixins/tconstruct/horizon-mixin.toml').read_text(encoding='utf-8'))
    rows = collections.defaultdict(list)
    unmapped = []
    for inj in manifest['injection']:
        fn = inj['target'].split('::')[-1]
        module, fabric = classify(fn, inj['id'])
        if module is None:
            unmapped.append((inj['id'], fn))
        else:
            rows[(module, fabric)].append((inj['id'], fn, inj.get('kind', '')))
    total = sum(len(v) for v in rows.values())
    L = ['# TConstruct hook map: current injections to Fabric-API-like PlutoLib modules', '',
         'Generated by `tools/map_hooks_to_fabric_api.py`; do not edit by hand. Each row is a legacy direct injection into Horizon that a PlutoLib API should replace.', '',
         f'{total} of {total + len(unmapped)} injections mapped; {len(unmapped)} unmapped.', '',
         '| PlutoLib module | Fabric API counterpart | Injections |', '|---|---|---|']
    for (module, fabric), v in sorted(rows.items(), key=lambda kv: -len(kv[1])):
        L.append(f'| {module} | {fabric} | {len(v)} |')
    for (module, fabric), v in sorted(rows.items(), key=lambda kv: -len(kv[1])):
        L += ['', f'## {module} ({len(v)})', '', f'Fabric: {fabric}', '', '| Injection | Host function | Kind |', '|---|---|---|']
        for ident, fn, kind in sorted(v):
            L.append(f'| {ident} | {fn} | {kind} |')
    if unmapped:
        L += ['', f'## UNMAPPED ({len(unmapped)})', '', '| Injection | Host function |', '|---|---|']
        for ident, fn in sorted(unmapped):
            L.append(f'| {ident} | {fn} |')
    a.out.write_text(chr(10).join(L) + chr(10), encoding='utf-8', newline='')
    print(f'mapped {total}, unmapped {len(unmapped)}')
    for module, _ in sorted(rows, key=lambda k: -len(rows[k])):
        pass
    print({m: len(v) for (m, _), v in rows.items()})
    if unmapped:
        print('unmapped:', unmapped[:40])


if __name__ == '__main__':
    sys.exit(main())
