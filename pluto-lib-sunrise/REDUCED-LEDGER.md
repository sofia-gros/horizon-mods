# REDUCED-LEDGER — entries removed from the shipped manifest for v1.0.0

Owner decision (2026-11-27): PlutoLib v1.0.0 ships in **reduced form**.
The Sunrise package gate (`cargo sunrise-mod check` / `build` /
release) refuses a package whose weave plan has any failing entry,
with no tolerance mechanism (see `~/workspace/sunrise-plan-gate/
VERDICT.md`). At the pinned SDK **ba1f24e4ad41246b558230aa665539dc768b1287**
(game 26.3), 75 of the manifest's 379 entries fail the plan. Those
75 entries were removed from `mod/sunrise-mod.toml`; their verbatim
manifest text is held in `mod/sunrise-quarantine.toml` (the tree's
existing quarantine precedent), and their helper source files remain
in `mod/src/` untouched (the tool stages only listed helpers).

Effect: the 75 surfaces below stay callable through the PlutoLib API
and registrations succeed, but the woven application points are not
installed, so the behaviours are **inert** in v1.0.0. They return in
a later PlutoLib release as their gating upstream items land (the
floor-only dependency model lights consumers up unmodified). The
consumer-facing version of this list is
`~/workspace/sunrise-reduced-build/LIMITATIONS.md`.

Error classes: NR = `E_MIXIN_NOT_REPLACEABLE`, TM =
`E_MIXIN_TARGET_MISSING`, MD = `E_MIXIN_MODULE_DESTINATION`
(shortened in the table).

| id | kind | error | target / destination | gating upstream item |
|---|---|---|---|---|
| `advancement-item-predicate` | mixin | NR | `sunrise_client_lib::advancements::item` | sunrise#5952 — custom item predicate kinds for advancements (and loot) |
| `advancement-tags-clear` | mixin | NR | `sunrise_client_render::chunk_map::ChunkMap::clear` | sunrise#5932 — per-entity/per-level instance identity tokens (the per-instance state these rows clear has no move-stable identity at this pin) |
| `crafting-match-veto` | mixin | NR | `sunrise_client_lib::crafting::Recipes::assemble_if_matches` | sunrise#5946 — scan-level hook for crafting matches (veto, special-recipe providers, placement-definition veto) |
| `crafting-special` | mixin | NR | `sunrise_client_lib::crafting::Recipes::find_with` | sunrise#5946 — scan-level hook for crafting matches (veto, special-recipe providers, placement-definition veto) |
| `drops` | mixin | NR | `sunrise_items::items::Items::correct_for_drops` | sunrise#5945 — replaceable callers or hooks for item table functions (mining_speed / correct_for_drops / load_with) |
| `landing-state-clear` | mixin | NR | `sunrise_client_render::chunk_map::ChunkMap::clear` | sunrise#5932 — per-entity/per-level instance identity tokens (the per-instance state these rows clear has no move-stable identity at this pin) |
| `max-damage` | mixin | NR | `sunrise_client_lib::lan::guest::stack_to_network` | sunrise#5947 — guest stack network conversion extension points / decoded-baseline accessor for Wire |
| `movement-table` | mixin | NR | `sunrise_world::movement_properties::MovementProperties<'_>::bind` | sunrise#5934 — reach for MovementProperties::bind |
| `pluto-advancement-tags-clear` | mixin | NR | `sunrise_client_render::chunk_map::ChunkMap::clear` | sunrise#5932 — per-entity/per-level instance identity tokens (the per-instance state these rows clear has no move-stable identity at this pin) |
| `pluto-attachment-transient` | module | MD | `crates/sunrise-world/src/attachments.rs` | no dedicated filing — module destination refused at this pin (crates/sunrise-world/src/attachments.rs); Attachments Descriptor/Key re-design surface (GAPS section C2) |
| `pluto-block-after-fall` | mixin | NR | `sunrise_client_lib::motion::<MotionWorld<'_,'_,'_> as EntityMoveWorld>::restitute` | sunrise#5074 — delivered upstream (PR #6073, living_travel_moved); returns at next re-pin + connection |
| `pluto-block-entity-revision` | mixin | NR | `sunrise_client_render::chunk::block_entity::ModBlockEntity::revision` | sunrise#5961 — invalidate a mod block entity kept geometry on client-only data change |
| `pluto-block-facts-publish` | mixin | NR | `sunrise_client::game::world_rest::load_world_rest` | sunrise#5963 — publish from Resources when world resources load (block-facts table) |
| `pluto-block-fall-damage` | mixin | NR | `sunrise_client_lib::landing::MotionWorld<'_,'_,'_>::mod_block_fall` | sunrise#5939 — entity-class query overrides (attack damage, fall calculation, cause-fall) |
| `pluto-block-inside-motion` | mixin | NR | `sunrise_client_lib::motion::MotionWorld<'_,'_,'_>::check_inside_blocks` | sunrise#5076 — entity_inside / inside-motion surface (open upstream; join comment posted) |
| `pluto-break-context-head` | mixin | TM | `sunrise_client::game::native_hooks::Game<'_>::native_destroy_block` | sunrise#5966 — block destroy event with context |
| `pluto-crafting-match-veto` | mixin | NR | `sunrise_client_lib::crafting::Recipes::assemble_if_matches` | sunrise#5946 — scan-level hook for crafting matches (veto, special-recipe providers, placement-definition veto) |
| `pluto-creative-rebuild` | mixin | NR | `sunrise_client_lib::creative::Creative::rebuild` | sunrise#5950 — creative tabs defined in code |
| `pluto-effect-icon-head` | mixin | TM | `sunrise_client::game::native_hooks::Game<'_>::native_hud` | no dedicated filing — HUD callback surface (GAPS section A2) |
| `pluto-entity-particle-kind` | mixin | NR | `sunrise_client_lib::mobs::cube::landing_splash` | sunrise#5937 — particle-add interception / item-particle factory registry |
| `pluto-entity-spawn-add` | mixin | NR | `sunrise_client::game::natural_spawning::Game<'_>::add_spawned` | sunrise#5933 — spawn completion primitives |
| `pluto-entity-spawn-conversion` | mixin | NR | `sunrise_client::game::creatures::Game<'_>::convert_to` | sunrise#5933 — spawn completion primitives |
| `pluto-entity-spawn-converted` | mixin | TM | `sunrise_client::game::creatures::Game<'_>::finalize_converted` | sunrise#5933 — spawn completion primitives |
| `pluto-entity-spawn-natural` | mixin | TM | `sunrise_client::game::natural_spawning::Game<'_>::finalize_spawned` | sunrise#5933 — spawn completion primitives |
| `pluto-entity-spawn-runtime` | mixin | TM | `sunrise_client::game::creatures::Game<'_>::spawn_mob_internal` | sunrise#5933 — spawn completion primitives |
| `pluto-hanging-frame-from-nbt` | mixin | TM | `sunrise_client::entity_nbt::hanging::item_frame_from_nbt` | no dedicated filing — host re-derivation: the target functions are absent from the pinned function index |
| `pluto-hanging-frame-hit` | mixin | TM | `sunrise_client::game::frames::Game<'_>::hit_frame` | no dedicated filing — host re-derivation: the target functions are absent from the pinned function index |
| `pluto-head-loot-gate` | mixin | NR | `sunrise_client_lib::mobs::drops::roll` | sunrise#5941 — death-loot callback context (drop construction, head-claim verbs) |
| `pluto-item-after-fall` | mixin | NR | `sunrise_client_lib::entities::ItemEntity::tick_move_before_block_effects` | sunrise#5074 — delivered upstream (PR #6073, living_travel_moved); returns at next re-pin + connection |
| `pluto-item-color-layers` | mixin | NR | `sunrise_client_lib::item_render::ItemRenderer::resolve_with_rule` | sunrise#5951 — per-stack item retexture (baked item layers) |
| `pluto-item-enchantability` | mixin | NR | `sunrise_items::enchant::Enchantments::enchantable_of` | sunrise#5945 — replaceable callers or hooks for item table functions (mining_speed / correct_for_drops / load_with) |
| `pluto-item-ender-mask` | mixin | NR | `sunrise_client_lib::mobs::enderman::stared_at` | sunrise#5948 — delivered upstream (PR #5982, gaze_disguise); returns at next re-pin + connection |
| `pluto-item-indestructible` | mixin | TM | `sunrise_client::entities::ItemEntity::environment_tick` | sunrise#5082 — implementation PR #5986 in review; returns when it merges and ships in an SDK + connection |
| `pluto-item-inside-motion` | mixin | NR | `sunrise_client::game::entity::Game<'_>::tick_item_with_block_contacts` | sunrise#5076 — entity_inside / inside-motion surface (open upstream; join comment posted) |
| `pluto-item-particle-factory` | mixin | TM | `sunrise_client::game::native_client_particles::Game<'_>::native_add_particle` | sunrise#5937 — particle-add interception / item-particle factory registry |
| `pluto-item-predicate` | mixin | NR | `sunrise_client_lib::advancements::item` | sunrise#5952 — custom item predicate kinds for advancements (and loot) |
| `pluto-item-prototypes` | mixin | NR | `sunrise_items::items::Items::load_with` | sunrise#5945 — replaceable callers or hooks for item table functions (mining_speed / correct_for_drops / load_with) |
| `pluto-living-attributes-amount` | mixin | NR | `sunrise_client_lib::mobs::finalize::modifier_amount` | sunrise#5938 — participation in the live attribute computation |
| `pluto-living-attributes-base` | mixin | NR | `sunrise_client_lib::mobs::finalize::set_base` | sunrise#5938 — participation in the live attribute computation |
| `pluto-living-attributes-compute` | mixin | NR | `sunrise_client_lib::mobs::finalize::compute` | sunrise#5938 — participation in the live attribute computation |
| `pluto-living-attributes-has` | mixin | NR | `sunrise_client_lib::mobs::finalize::has_modifier` | sunrise#5938 — participation in the live attribute computation |
| `pluto-living-attributes-remove` | mixin | NR | `sunrise_client_lib::mobs::finalize::remove_modifier` | sunrise#5938 — participation in the live attribute computation |
| `pluto-living-cause-fall` | mixin | NR | `sunrise_client_lib::mobs::living::Mob::cause_fall_damage_at` | sunrise#5939 — entity-class query overrides (attack damage, fall calculation, cause-fall) |
| `pluto-living-drops-game` | module | MD | `crates/sunrise-client/src/game/creatures.rs` | sunrise#5941 — death-loot callback context (drop construction, head-claim verbs) |
| `pluto-living-entity-attack` | mixin | NR | `sunrise_client_lib::mobs::cube::attack_damage` | sunrise#5939 — entity-class query overrides (attack damage, fall calculation, cause-fall) |
| `pluto-living-entity-fall` | mixin | NR | `sunrise_client_lib::mobs::living::Mob::cause_fall_damage_at` | sunrise#5939 — entity-class query overrides (attack damage, fall calculation, cause-fall) |
| `pluto-living-preview-flush` | mixin | TM | `sunrise_client::game::native_hooks::Game<'_>::native_screen_draw` | no dedicated filing — render-flush surface (ScreenRenderPostEvent sufficiency unverified at this pin) |
| `pluto-map-face-flush` | mixin | TM | `sunrise_client::game::native_hooks::Game<'_>::native_hud` | sunrise#5962 — draw a filled map face through the mod API GUI drawing surfaces |
| `pluto-mob-after-fall` | mixin | NR | `sunrise_client_lib::mobs::living::Mob::do_move` | sunrise#5074 — delivered upstream (PR #6073, living_travel_moved); returns at next re-pin + connection |
| `pluto-mob-effect-apply` | mixin | TM | `sunrise_client::game::native_hooks::Game<'_>::native_living_incoming_damage_v2` | sunrise#5942 — tick flush point for queued hit/inside consequences |
| `pluto-mob-effect-apply-tick` | mixin | NR | `sunrise_client::game::lan_attributes::Game<'_>::update_player_attributes` | sunrise#5942 — tick flush point for queued hit/inside consequences |
| `pluto-mob-fall-damage` | mixin | NR | `sunrise_client_lib::mobs::living::Mob::mod_block_fall` | sunrise#5939 — entity-class query overrides (attack damage, fall calculation, cause-fall) |
| `pluto-mob-inside-motion` | mixin | NR | `sunrise_client_lib::mobs::ai::effects_from_blocks` | sunrise#5076 — entity_inside / inside-motion surface (open upstream; join comment posted) |
| `pluto-path-node-type` | mixin | NR | `sunrise_client_lib::mobs::path::MobWorld<'_>::path_type_from_state` | sunrise#5943 — path-type classification for arbitrary block states |
| `pluto-player-visibility` | mixin | NR | `sunrise_client_lib::mobs::PlayerView::visibility_percent` | sunrise#5944 — delivered upstream (PR #5977, LivingRef::player); returns at next re-pin + connection |
| `pluto-reload-before-tables-reload` | mixin | TM | `sunrise_client::game::world_entry::Game<'_>::reload_world_tables` | sunrise#5949 — reload listeners (listener objects, registry source, two-phase apply, pre-tables phase) |
| `pluto-reload-registry` | mixin | TM | `sunrise_client::game::native_reload_listeners::Game<'_>::native_reload_listeners` | sunrise#5949 — reload listeners (listener objects, registry source, two-phase apply, pre-tables phase) |
| `pluto-retexture-layer` | mixin | NR | `sunrise_client_render::mesher::Mesher<'_>::face` | sunrise#5960 — Mesher::face replaceable / per-face seam |
| `pluto-structure-generation-start` | mixin | NR | `sunrise_world::worldgen::generator_decoration::Decoration::decorate` | sunrise#5959 — worldgen structure-start hooks |
| `pluto-structure-natural-start` | mixin | NR | `sunrise_world::worldgen::generator_structures::StructureState::build_starts` | sunrise#5959 — worldgen structure-start hooks |
| `pluto-structure-saved-piece` | mixin | NR | `sunrise_world::worldgen::generator_structures::StructureState::saved_pool_piece` | sunrise#5959 — worldgen structure-start hooks |
| `pluto-structure-sources` | mixin | NR | `sunrise_world::worldgen::generator_structures::StructureState::new` | sunrise#5959 — worldgen structure-start hooks |
| `pluto-structure-start` | mixin | NR | `sunrise_world::worldgen::generator_structures::StructureState::structure_start` | sunrise#5959 — worldgen structure-start hooks |
| `protection-arm` | mixin | NR | `sunrise_client_lib::health::<Environment<'_> as ArmorWorld>::server_level` | sunrise#5940 — damage-pipeline stage for armor protection |
| `protection-kernel` | mixin | NR | `sunrise_world::combat_rules::after_magic` | sunrise#5940 — damage-pipeline stage for armor protection |
| `protection-player-scope` | mixin | NR | `sunrise_client_lib::health::PlayerMotion::hurt_caused_by` | sunrise#5940 — damage-pipeline stage for armor protection |
| `protection-project` | mixin | NR | `sunrise_client_render::effects::StatusFlags::protection` | sunrise#5940 — damage-pipeline stage for armor protection |
| `recipe-placement-definition` | mixin | NR | `sunrise_client_lib::crafting::Recipes::placement` | sunrise#5946 — scan-level hook for crafting matches (veto, special-recipe providers, placement-definition veto) |
| `saved-book-clear` | mixin | NR | `sunrise_client_render::chunk_map::ChunkMap::clear` | sunrise#5932 — per-entity/per-level instance identity tokens (the per-instance state these rows clear has no move-stable identity at this pin) |
| `send-custom-data` | mixin | TM | `sunrise_client::lan::hub::Hub::stack` | no dedicated filing — network custom-data event surface (NetFromClient/ServerEvent sufficiency unverified at this pin) |
| `speed` | mixin | NR | `sunrise_items::items::Items::mining_speed` | sunrise#5945 — replaceable callers or hooks for item table functions (mining_speed / correct_for_drops / load_with) |
| `station-ingredient-parser` | mixin | NR | `sunrise_client_lib::stations::ingredient` | sunrise#5953 — station ingredient parsing (whole-value parser evaluation) |
| `tool-decode` | mixin | NR | `sunrise_client_lib::item_sync::from_network` | sunrise#5947 — guest stack network conversion extension points / decoded-baseline accessor for Wire |
| `tool-encode` | mixin | NR | `sunrise_client_lib::lan::guest::stack_to_network` | sunrise#5947 — guest stack network conversion extension points / decoded-baseline accessor for Wire |
| `wire-read` | module | MD | `crates/sunrise-items/src/item_data.rs` | sunrise#5947 — guest stack network conversion extension points / decoded-baseline accessor for Wire |

## Return procedure

When an SDK ships whose surface covers a gating item: re-pin
(SUNRISE.md procedure), restore the entry's block from
`mod/sunrise-quarantine.toml` into `mod/sunrise-mod.toml`, wire the
native route per the connect-prep plan where one now exists, re-run
the plan (zero-new-failing-ids rule), and delete the row here.
