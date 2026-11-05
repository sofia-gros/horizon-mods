# Filled-tank acquisition goals

Part of high-priority #379. Stacked on the acquisition chain through #850.

- **Blaze:** any one of the eight source Seared/Scorched fuel tanks/gauges (exactly 4000 mB) or ingot tanks/gauges (exactly 4320 mB), filled with Blazing Blood.
- **Manyullyn Lanterns:** one stack of at least 64 Seared Lanterns or at least 64 Scorched Lanterns, each holding exactly 50 mB Molten Manyullyn. Separate smaller stacks or a mixed sum do not qualify.

Both retain their source criterion names, OR requirement grouping, Foundry structure parent, titles and descriptions. The source icon's filled-tank NBT is omitted because the host renders only the item icon. This does not weaken the criteria.

## Native predicate boundary

The source JSON uses legacy Forge `tank.Amount` / `tank.FluidName` SNBT. The host's vanilla item matcher does not evaluate that data. Instead, these two goals use an explicit cancellable `tconstruct:portable_tank` predicate and the real native item snapshot at `minecraft:custom_data` → `tconstruct:tank` → `amount` / `fluid`. Existing portable casting and container transfer producers already write this representation; they are not modified here.

The bridge requires exactly one of the ten supported item IDs, a nonempty stack, an optional integer min/max count range, one of the two goal fluids and an exact positive amount within that item's capacity. Unknown predicate fields and malformed forms fail closed rather than falling through to vanilla item-only matching. Fluid IDs are decoded without lossy UTF-16 replacement. Missing/empty data, wrong fluid, partial/overfilled tanks, legacy-only and mixed Forge/native owned fields do not qualify. A present `fluid_tag` must be a compound. Unrelated native extension fields and compound fluid metadata remain allowed, matching the source NBT subset semantics; the predicate does not mutate any data.

## Dependencies and limits

Requires host PR [#38](https://github.com/sofia-gros/horizon/pull/38) for item custom-data snapshots; [#41](https://github.com/sofia-gros/horizon/pull/41) additionally triggers inventory evaluation after metadata-only changes. It does not add a generic SNBT matcher, arbitrary FluidHandler support, fluid filling, new recipes, drops, world spawning or acquisition bypasses.

The immediate Foundry structure parent is present. Its Alloyer ancestor remains absent, so the full Foundry display ancestry is still incomplete. Natural Blazing Blood production, full in-world acquisition and UI behavior are not established by these predicates. Existing fluid and machine branches require later combined integration. No transfer codec/runtime is changed here.

Exact source files and hashes are under `tests/fixtures/filled_tank`. Focused Rust predicate cases and Python source/package/wiring cases were added. Only brief static review and whitespace checking were performed. Tests, builds, CI inspection, version checks, gameplay, save/reload and LAN validation were deliberately deferred.
