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

## Gate-iteration addendum (2026-11-27, reduced-build lane)

The 304-entry reduced manifest still failed the tool's woven compile (the
guarded build had never been run on this tree). 188 further entries were
quarantined during gate iteration; their blocks are appended to
`mod/sunrise-quarantine.toml` under batch headers, and the same return
procedure applies. Batch reasons:

- **Gate round 2 (duplicate generations dropped; staging/mirror repairs)**
- **Gate rounds 3-4 (structure/tree worldgen family written against the Horizon host's flat worldgen; contact-sweep/motion restitution/dynamic screen slots backing absent; spawn-finalize family; living drops game side; current-book API never written; fnr private-macro wall on the two explosion mixins)**
- **Gate round 5 batch 4 (host-private or absent types and functions: menu Role/Slot/PLAYER, finalize attribute refresh, mod_content Overlay, recipe-book Known, analyse_shape, orphan-rule crafting impl, advancement parse)**
- **Gate round 6 batch 5 (body-level host drift found by the first woven compile: struct field drift, private Game methods in fnr copies, inherent impl Game blocks illegal in the patched crate, ChunkMap state accessors; plus the reference cascade)**
- **Gate round 7 batch 6 (method-call dependents of batch 5: projectile impact game, sleeves takeover, advancement tag index/predicate, seat-player)**
- **Gate round 8 batch 7 (fnr-copy and helper dependents: creative click/pages, item use start/finish, menu switch flushes, shear saddle, living drops head, item wear, inventory advancement components, dynamic screen slots data, fluid assemble, break-complete return)**
- **Gate round 9 (pluto-reload-before-tables-open: native_reload_listeners does not exist in Sunrise)**

| id | class | batch | gate |
|---|---|---|---|
| dynamic-screen-slots | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| inventory-export | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| stack-snapshot | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| native-creative-potion-facts | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| advancement-events | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| advancement-event-source | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| advancement-tag-state | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| advancement-tags | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| component-view | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| crafting-match | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| pluto-menu-actions-guest | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| pluto-living-attributes-reload | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| pluto-block-exp | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| pluto-entity-climb-record | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| pluto-entity-particle-host-event | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| pluto-armor-worn-layers | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| pluto-armor-stand-worn-layers | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| draw-buttons | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| recipe-placement-click | gate | Gate round 2 | woven compile at SDK ba1f24e4 |
| pluto-entity-spawn-game | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-spawn-placement-game | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-mob-inside | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-item-inside | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-contact-sweep | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-mob-landing | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-block-motion | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-contact-sound | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-structure-context | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-structure-generation | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-structure-generation-placement | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-structure-natural | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-structure-piece | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-structure-registered-piece | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-structure-markers | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-structure-level | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-structure-command | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-tree-placement | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-tree-catalog | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-tree-level | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| dynamic-screen-slots-api | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| current-book | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-living-drops-game | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-spawn-placement-permit | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-entity-spawn-spawner | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-entity-spawn-spawner-add | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-entity-spawn-raid | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-entity-spawn-structure | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-structure-command-head | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| dynamic-screen-slots-update | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-hanging-frame-explosion-return | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| equipment-explosion-replacement | gate | Gate rounds 3-4 | woven compile at SDK ba1f24e4 |
| pluto-menu-equipment | gate | Gate round 5 batch 4 | woven compile at SDK ba1f24e4 |
| pluto-living-attribute-ops | gate | Gate round 5 batch 4 | woven compile at SDK ba1f24e4 |
| pluto-living-attributes-join-module | gate | Gate round 5 batch 4 | woven compile at SDK ba1f24e4 |
| pluto-item-placement-export | gate | Gate round 5 batch 4 | woven compile at SDK ba1f24e4 |
| pluto-block-path | gate | Gate round 5 batch 4 | woven compile at SDK ba1f24e4 |
| pluto-block-view-shapes | gate | Gate round 5 batch 4 | woven compile at SDK ba1f24e4 |
| pluto-crafting-match | gate | Gate round 5 batch 4 | woven compile at SDK ba1f24e4 |
| advancement-projection | gate | Gate round 5 batch 4 | woven compile at SDK ba1f24e4 |
| advancement-projection-world | gate | Gate round 5 batch 4 | woven compile at SDK ba1f24e4 |
| recipe-index-patch | gate | Gate round 5 batch 4 | woven compile at SDK ba1f24e4 |
| pluto-menu-equipment-guest-load | gate | Gate round 5 batch 4 | woven compile at SDK ba1f24e4 |
| pluto-menu-equipment-slot | gate | Gate round 5 batch 4 | woven compile at SDK ba1f24e4 |
| pluto-living-attributes-join | gate | Gate round 5 batch 4 | woven compile at SDK ba1f24e4 |
| pluto-reload-registry-source | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-mob-split-game | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-entity-hurt-game | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-block-entity-access | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-block-entity-client-tick-module | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-block-entity-client-level | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-block-fall | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-block | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-block-level | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-sleeves-takeover-entity | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-contact-effects | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-block-inside | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-worldgen-level | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-retexture-module | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-living-drop-state-module | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-entity-push-access | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-entity-teleport-access | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-item-particles | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-item-colors | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-retexture-item | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-living-preview-api | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-living-preview | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-menu-switch-api | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-menu-switch | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-map-face-api | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-map-face | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-explosion-flush | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-brush-flush | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-use-stop-flush | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-component-view | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-stack-storage | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-advancement-events | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-advancement-tag-source | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-advancement-tags-sink | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-hanging-game | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-fishing-game | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-projectile-flight-game | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-map-tick | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-effect-icon | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-throw-sound-game | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| tooltip-policy | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| stack-storage | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| landing-state-api | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| cloud-factory | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| cloud-insertion | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| protection-player-api | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| motion-query | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| world-fluid | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| protection-native-tests | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| equipment-game-dispatch | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| block-feedback | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| protection-provider-facts | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| equipment-view | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| armor-wear-observation | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| recipe-book-invalidation | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| guide-text | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| recipe-book-preview | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| saved-book-capture | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| saved-book-world | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| recipe-placement | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| recipe-placement-attempt | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| station-ingredient | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-attachment-transient | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-contact-sound | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-mob-split-head | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-block-entity-client-level-replace | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-block-entity-client-level-unload | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-world-contact-begin | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-world-contact-flush | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-cooking-recipes-tick | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-sleeves-takeover-entity-return | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-contact-sound-flush | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-head-wave-reset | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-entity-drain | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-advancement-event-drain | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-hanging-frame-place | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-hanging-frame-interact | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-hanging-frame-tick | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-hanging-frame-analog | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-hanging-frame-explosion-head | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-hanging-frame-drops | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-hanging-frame-set-item | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-hanging-stand-place | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-hanging-stand-break | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-hanging-stand-dispense | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-hanging-stand-command | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-fishing-tick-head | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-projectile-spawn-head | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-projectile-flight-return | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-map-tick-head | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-map-tick-return | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-throw-sound | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| durability-tooltip | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| tooltip | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| items-b2-tooltip | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| world-fluid-use | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| advancement-event-drain | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| equipment-ordinary-replacement | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| armor-wear-observation-replacement | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| saved-book-before-open | gate | Gate round 6 batch 5 | woven compile at SDK ba1f24e4 |
| pluto-sleeves-takeover | gate | Gate round 7 batch 6 | woven compile at SDK ba1f24e4 |
| pluto-advancement-tag-state | gate | Gate round 7 batch 6 | woven compile at SDK ba1f24e4 |
| pluto-projectile-impact-game | gate | Gate round 7 batch 6 | woven compile at SDK ba1f24e4 |
| pluto-sleeves-takeover-return | gate | Gate round 7 batch 6 | woven compile at SDK ba1f24e4 |
| pluto-entity-seat-player | gate | Gate round 7 batch 6 | woven compile at SDK ba1f24e4 |
| pluto-advancement-tag-predicate | gate | Gate round 7 batch 6 | woven compile at SDK ba1f24e4 |
| pluto-advancement-tag-index | gate | Gate round 7 batch 6 | woven compile at SDK ba1f24e4 |
| pluto-projectile-impact-head | gate | Gate round 7 batch 6 | woven compile at SDK ba1f24e4 |
| advancement-tag-predicate | gate | Gate round 7 batch 6 | woven compile at SDK ba1f24e4 |
| advancement-tag-index | gate | Gate round 7 batch 6 | woven compile at SDK ba1f24e4 |
| pluto-item-shear-saddle | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| pluto-item-use-start | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| pluto-item-use-finish | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| pluto-living-drops-head | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| dynamic-screen-slots-data-update | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| pluto-menu-switch-flush | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| pluto-menu-switch-guest-flush | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| pluto-item-wear | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| pluto-break-complete-return | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| pluto-inventory-advancement-components | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| prepare-pages | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| click-buttons | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| use | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| items-b2-use | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| inventory-advancement-components | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| fluid-render-before-assemble | gate | Gate round 8 batch 7 | woven compile at SDK ba1f24e4 |
| pluto-reload-before-tables-open | gate | Gate round 9 | woven compile at SDK ba1f24e4 |

Total quarantined during gate iteration: 191 entries. Shipped manifest: 116 entries (21 mixins + 95 modules).

## Addendum 2026-11-27 - package file-count relocation

The Sunrise native package format admits at most 511 source files
(smod crate manifest check: source_count > 511 is refused on both
archive and folder intake). The reduced package carried 530 source
files, so the launcher refused pluto_lib.smod at install/switch.
On 2026-11-27, 46 quarantined entries' source files were moved from
mod/src/ to mod/quarantine-src/ (the package walk only packs src/**,
so they are no longer counted); the paths in sunrise-quarantine.toml
were updated to match. No file contents changed and the shipped
manifest is untouched. Return procedure addition: when restoring a
relocated entry, move its file back from mod/quarantine-src/ to
mod/src/ together with the manifest entry. Shipped-package source
count after the move: 484.
