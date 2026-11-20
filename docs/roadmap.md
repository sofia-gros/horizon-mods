# Clean-port roadmap

Active from 2026-11-14. This replaces the archived implementation order and task workflow. The original roadmap was preserved in `legacy/2026-11-14/docs/roadmap.md`. Old inventories remain references only; neither `ported` nor `parked` proves the clean port works.

## Outcome and boundaries

Deliver the original in-scope Tinkers behavior on official Horizon through a reusable, independent PlutoLib. Tinkers has zero host injections and no private host dependency. PlutoLib owns native attachment and exposes general capabilities, never Tinkers rules. Owner clarification (2026-11-14): unsupported Mixins contexts are handled by PlutoLib-owned direct Horizon source changes, not by waiting for upstream; preserve official reference inputs and verify the patched build inputs separately. Exclude other-mod integration. Preserve creative paging, stack state, original assets' licences and source-defined behavior.

The current reconstruction creates an empty native entry point, NOT a playable replacement. Old behavior must not leak into it. Do not distribute a development package as a feature-complete release. Keep pins fixed during one verification unit; a later upstream update is a separate compatibility change.

## Execution order

The small dependency queue in `port-plan.json` is the operational source of work. `python tools/port.py next` selects an unblocked task deterministically. One integration task is active at a time; source-ready tasks with unresolved technical prerequisites are blocked, not completed. Do not resurrect the old class-status selector.

| Task | Deliverable | Exit condition |
|---|---|---|
| R0 Separate products | Hash-verified old Tinkers snapshot; clean entry point; boundary-aware package command; concise rules | No archive references/injections in the new bundle; package and rejection checks pass. This is infrastructure only. |
| R1 Establish trustworthy execution | Audit PlutoLib's public API/host link, launcher API dependency injection, existing build/package/CI entry points, pins and isolation | One canonical new-product build/run path; no old fallback; preserved CI checks are reconciled rather than silently removed. Record actual linkage and runtime evidence. |
| R3 Close dependency map (before R2) | Original-wide registrations and consumers -> behavior domains -> required capabilities; include later fluid/casting/entity/world requirements in shared API design | Finite Java-grounded scope for the first domain and subsequent domains, with prerequisites and no circular dependencies. No class-by-class prose. |
| R2 First large implementation domain (after R1 + R3) | Common foundations + tool construction/use: registration/data, materials/parts/tool calculations, stack state, required stations/inventories/screens, use/durability/repair, required modifier paths, persistence/sync and presentation | Complete material -> part -> tool -> use -> repair -> save/reload loop, relevant synchronization and presentation, with source review and authorized gameplay evidence. Neither one demonstration tool nor a small block behavior substitutes for this domain. |
| R4 Port by dependency | Implement one complete group at a time using the map | Required API + real adapter + Tinkers consumer + meaningful scenario; related save/reload/sync where applicable. |
| R5 Whole-product parity | Exercise interactions, persistence, resources and multiplayer; reconcile original content coverage | No unresolved in-scope behavior; independent source review and gameplay evidence for the shipped inputs; release gates pass. |

Owner-fixed strategy (2026-11-14): R1 and R3 precede R2. R3 may proceed while native execution is blocked. Small empty-entry linkage probes belong to R1, not to the first gameplay domain. Slime soil is deferred to the world/vegetation domain. Implement the general PlutoLib capabilities required by R2 with real host adapters, not an entire speculative API platform; keep original Tinkers rules in Tinkers. Internal subtasks follow dependencies, but completion/review and expensive native/game validation stay at coherent domain boundaries rather than per helper, injection or block. R4 is now an aggregate of the finite R4-fluids, R4-world and R4-gadgets domain tasks in the queue; their implementation must follow their recorded prerequisites. Do not change this strategy without an explicit owner decision; routine implementation choices need no approval.

### Dependency-map rules

Start from Java registrations and their actual consumers, not only package names or existing Rust. Cover world/vegetation/structures, entities, fluids, shared behavior/config/resources/networking, materials/tool construction/modifiers/combat, stations, smeltery/casting and client presentation. Identify essential Mantle/Forge/inherited vanilla semantics too.

Extract only the prerequisites required by the next feature group. For example equipment depends on working tool construction and necessary tool behavior; it is not complete merely because a mob holds a stack. World fluids used by structures are prerequisites, not something to park until a later alphabetic phase. Every class remains traceable, but class counts do not schedule work or determine completion.

Implement standard original-data paths first, then the feature's remaining accepted forms and edge semantics in a bounded pass. This changes order, not parity scope. Keep unsupported forms explicit; never silently approximate them. Never expand into unrelated features because a decoder exposes another possible format.

## Per-unit implementation contract

Before editing a group, record only:

- original entry points/data and source revision;
- user-visible result and affected save/network state;
- required public APIs and actual host callers;
- explicit prerequisites, permitted paths and missing edges;
- a small verification scenario and what would falsify correctness.

Then: read Java -> design the general API if absent -> implement its host adapter -> register the Tinkers consumer -> trace the actual call chain -> focused check -> source review -> consolidated native/game verification. A function used only by tests is not a connected feature. Reuse source-correct old algorithms/assets individually; do not bulk-copy old runtimes or tests.

Source review must settle the required semantics, API ownership, callback ordering, lifetime, actual caller reachability and save/sync responsibilities. Review does not waive source-integrity or attachment-contract checks. Native compilation checks types/attachment; gameplay checks outcomes. Neither substitutes for the other.

## Minimal verification, not a second project

Tests serve the port, not the reverse. Add a small test only when it can distinguish a plausible wrong implementation from the Java behavior. Test production functions/callers, not a duplicate simulation. Prefer one scenario covering a real state transition over a matrix of trivial getters. Never assert documentation wording or add tests to reach a number.

Run focused tests during edits, related regression checks when the integration unit closes, and broader checks before consolidated validation. Do not repeatedly run every doctest/static suite after each small change. A public API needs a short real example and its important limitation; existing huge documentation is not a template.

For native validation, group related source-reviewed changes into one binary and reuse it for scenarios. Do not rebuild unchanged inputs. Use isolated homes/worlds, never user saves. Inspect images when appearance is the behavior. Do not rename pure-test success to game verification.

Owner decision (2026-11-14): previous Phase completion/order restrictions are removed. Isolated official-source builds and game validation proceed for the current domain or R1 linkage work, without waiting for an unrelated domain or renewed Phase approval. Source-integrity, injection-ownership and actual attachment-contract checks remain mandatory. Resolve missing fingerprints against the pinned official source and generated plan; never invent approval, execute the archive or use user saves. A concrete technical blocker affects only dependent work.

## Status and evidence

Task states: `pending`, `in_progress`, `blocked`, `done`. A block names the concrete prerequisite and an alternative already considered. A completed task needs evidence appropriate to its kind: infrastructure checks for infrastructure; real-game results for gameplay. `source_ready` is an evidence stage, not gameplay completion. `parked`, “no API”, TODO comments and passing helper tests never count as completion.

Keep evidence short: input digest/revision, command/scenario, outcome, unresolved scope. Source changes invalidate affected runtime evidence; preserve historical results as historical. Do not reclassify all old classes as complete or import old game receipts as proof of the new product. Machine checks can reject missing records, not certify the truth of a human/AI claim.

## Continuous work without approval loops

Within the authorized scope, choose conventional file/API names, implement missing general APIs, run permitted checks, repair failures and continue immediately. Do not end a turn just because a helper or test is finished. Do not schedule a minute-long continuation instead of doing the next step. Background completion notifications are the primary wake signal; do other independent work meanwhile.

Ask only for an actual permission boundary or a decision that cannot safely be resolved from the source and requirements. Never ask again for already authorized local archive/switch work. Do not treat this as authorization to push, release, disable CI, spend money or bypass denied tools. When one task is blocked, select another ready task; if none exists, give one precise blocker report rather than repeatedly checking or requesting “continue”.

## Preservation and rollback

`legacy/2026-11-14/tconstruct/` retains the previous working tree, including uncommitted and untracked source. `snapshot.json` records source/resource hashes; build caches were moved but are not hashed. Original active documents were copied alongside it before replacement. The archive is not a release or runtime dependency.

Rollback must first preserve the NEW tree too, then restore the archived paths without overwrite. Do not use `git reset --hard`, delete snapshots, or assume Git contains the uncommitted work. The owner subsequently authorized commits, pushes, PR creation and merges on 2026-11-14, including while asleep. Preserve unrelated changes and snapshots; releases remain separately authorized.

## Guard implementation and remaining work

The initial `port.py` validates the new Tinkers product and packages an explicit input set. Its checks are structural, not a full Rust security parser or a parity oracle. R1 must integrate the checker into the existing authoritative build/CI paths without disabling checks, verify launcher linkage and retain old evidence separately. R3/R4 must extend the queue with concrete source-grounded groups. No claim is made that these later tasks are already implemented.

## R4-fluids: source inventory and unit order

Source: original revision `f4e77d43` (pinned in `tconstruct/compatibility.json`). Prerequisite R2 is still open; the units below that need no host feature may proceed meanwhile.

**What the original has.** `fluids/TinkerFluids` registers 72 fluids with source, flowing, bucket and block: slimes (earth, sky, ender, magma, ichor, venom), honey, four soups, potion and powdered snow (both unplaceable), seared and scorched stone, molten clay, glass, porcelain, obsidian, ender, soul, blazing blood, the gems and debris, about 17 vanilla-ingredient metals and alloys and about 27 more (tin, aluminum, lead, silver...) whose recipes name other mods' ingot tags, fiery liquid. 84 tconstruct, 45 forge and 1 mantle fluid tag files ship with them. `smeltery/TinkerSmeltery` has 58 block registrations and their block entities: melter, heater, scorched alloyer, smeltery and foundry controllers (`HeatingStructureBlockEntity` over `MultiblockCuboid`, tags `SMELTERY*`/`FOUNDRY*`), drains, ducts, chutes, tanks, casting table/basin/tank, faucet, channel, fluid cannons, gauges, lanterns, proxy tank, plus the seared/scorched building blocks, the copper can, tank items, 7 packets, 4 menus (3 screens) and 9 renderer classes. The content mod declares none of this (no fluid, smeltery block or cast item). The shipped recipe data is 1858 files outside `compat/` in `data/tconstruct/recipes` (copied to `recipes/fluids` by `tools/import_original_data.py`, with the fluid tags in `tinkering/fluid_tags`): melting 819 decoded, material melting 48, fuels 3, entity melting 32, alloys 24, casting 652, molding 58, material fluids 73, cast patterns 112.

**PlutoLib today** (pluto-lib/docs/api): `FluidRegistry` (attributes, tint, names), up to 16 saved tanks per block entity with `fluid`/`set_fluid` in tickers and Fabric-style storage for neighbours, item slots, menus, synced data. **Missing**: world blocks and buckets of a mod fluid (documented no-op), fluid stacks inside items (bucket, copper can, tank item), multiblock detection (reading a region of blocks against block tags from a controller), per-side tanks and capabilities for faucets, channels and ducts, fluid drawing in block entity renderers and screens, and a hook to melt an entity the smeltery hurts.

**Units, in order.**
1. Done (pure): `fluid_values`, `heating`, `fluid_ingredient`, `melting_recipes`, `casting_recipes`, `fluid_recipes`: amounts, melting/cooling arithmetic, ore rates, damageable scaling, slot heating and fuel state machines, structure capacity and fuel cost, and decoding plus matching of every shipped recipe type (alloy matching/mixing included). Left out by name: tool casting, part swapping, tipping (35 recipes, unit 7) and two ingredients (`mantle:potion_display`, `tconstruct:block_tag`) that need item components and the item-to-block link.
2. Done (source-ready, not game-verified): the 72 `FluidDef`s (`fluids.rs` over the generated `fluid_defs.rs`: temperature, density, viscosity, luminance and tint as TinkerFluids' `FluidType.Properties` and the generated mantle fluid textures give them; names are the `fluid.tconstruct.*` lang keys), registered through `FluidRegistry` from `hooks()`; PlutoLib fluid tags (`reload_registry("minecraft:fluid")`: native plus defined fluids with the tags the host loader reads from the root and the world's packs; Tinkers keeps the snapshot in `common::fluid_snapshot`); and the 84 `tconstruct` fluid tag files in `data/tconstruct/tags/fluid` (flowing variants dropped, forge/mantle references inlined, unshipped ones optional; the byte-exact 130 files stay in `tinkering/fluid_tags` for the recipes). Source-ready only: the native module calls `items::load_tags(root, "fluid", overlay)` and was not built against the pinned host. Left for later: bottles and food items (need item components and `FluidStack` in items), world blocks, flowing fluids and buckets (wait for the host).
3. Melter, heater and the solid-fuel path: one tank, 3 slots, `MeltingSlot`/`Fuel` ticks every 4 ticks, menu and screen. Needs only existing tanks, menus and tickers.
4. Casting table and basin with cast items, molding and sand casts: cooling timer, `ItemSlot` cast/output, fluid fill from a tank. Needs fluid stacks in items for filling recipes.
5. Smeltery: pure `MultiblockCuboid` port over a block lookup closure, then PlutoLib region/block-tag reads, `SmelteryTank` (multi-fluid), melting inventory, alloying, entity melting.
6. Foundry, alloyer, faucet, channel, drains/ducts, fluid cannons, gauges, lanterns, tank items, copper can.
7. Tool casting, part swapping, tipping and the modifier casting recipes, on the finished tool domain.
