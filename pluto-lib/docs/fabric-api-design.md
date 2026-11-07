# PlutoLib specification (draft for owner approval)

Roadmap step 1. Principles: [design-principles.md](design-principles.md). Evidence: [fabric-api-hook-map.md](fabric-api-hook-map.md)
(the 290 legacy TConstruct injections mapped to Fabric API modules by `tools/map_hooks_to_fabric_api.py`; the mapping is by host
function and injection id: a heuristic for review, not a final design).

Status: **draft**. Nothing here is implemented. It becomes binding when the owner approves it.

Revision 2 incorporates the owner's answers: Rust-idiomatic naming (§1), a single repository with independent per-product releases (§3),
and the dependency mechanism Horizon already provides (§2).

## 1. Conventions

**Mimic Fabric API where it fits, be idiomatic Rust where it does not.** Fabric API is the model for module boundaries, concepts, event
names and behaviour so that a Fabric mod developer recognises everything. It does not have to be copied literally: wherever Java idioms
fight Rust, use the Rust idiom and note the difference next to the API.

| Fabric API (Java) | PlutoLib (Rust) | Note |
|---|---|---|
| artifact `fabric-lifecycle-events-v1` | module `pluto_lifecycle_events_v1` inside the `pluto_lib` API | one API package, modules mirror Fabric modules |
| `ServerTickEvents.END_SERVER_TICK.register(cb)` | `ServerTickEvents::END_SERVER_TICK.register(cb)` | same names; the constant is an associated `static` |
| functional interface `UseItemCallback` | trait `UseItemCallback` implemented by closures (`Fn(..) -> ActionResult`) | closures instead of lambdas |
| `ActionResult.PASS / SUCCESS / FAIL` | enum `ActionResult` with the same variants | |
| exceptions, `null` | `Result`, `Option` | idiomatic |
| overloads | distinct method names or builder methods | Rust has no overloads |
| `FabricBlockSettings.create().strength(1.5f)` | `FabricBlockSettings::new().strength(1.5)` | snake_case methods |
| `Identifier` | `Identifier` | |
| mutable event phases | `Event::register_with_phase(phase, cb)` | |

**Always the full API.** Where the pinned Horizon or Horizon Mixins cannot do something yet, the call exists and its body is a stub
(a no-op, `false` or the documented default). A *support matrix* (`pluto-lib/docs/support-matrix.md`, created with the first module) lists
every stub, why, and which host change unblocks it.

## 2. How PlutoLib is built on Horizon (found in Horizon's own documentation)

Horizon already provides what a Fabric-API-like library needs; PlutoLib uses it instead of inventing parallel mechanisms.

1. **A mod is a "trait mod".** A bundle declares `[api] path = "engine"` in `horizon-mixin.toml`; the package exports
   `pub fn hooks() -> Box<dyn horizon_client::mod_api::Hooks>`; the launcher links all trait mods into the game and calls `hooks()` in
   dependency order. That is the entry point (the counterpart of Fabric's `ModInitializer`).
2. **Mods depend on mods with `[dependencies]`.** In `horizon-mixin.toml`: `pluto_lib = "^1"` (Cargo semver, checked against the dependency's
   `[mod] version`; `optional = true` is supported). The launcher adds the Cargo path dependency, using the mod id as the crate alias
   (hyphens become underscores). A dependent calls the functions and statics published by the dependency's `[api]` package. Missing
   dependencies, incompatible versions and cycles stop the build and name the mods. This is the Fabric API installation model: PlutoLib is an
   independent mod; Tinkers declares `pluto_lib = "^1"` and calls `pluto_lib::...`. (Both PlutoLib and every dependent need an `[api]` package.)
3. **Stable, additive "native hook points".** `crates/horizon-client/src/game/native_hooks.rs` holds hundreds of empty functions the game
   calls at fixed places (the `native_*` targets), kept stable across releases and only ever added to; each addition raises
   `NATIVE_HOOKS_API`. A mod that hooks only these names `"hooks": N` in its `horizon-native.json` instead of pinning an exact Horizon
   commit, and builds on any Horizon whose `NATIVE_HOOKS_API` is at least N. The `Hooks` trait (generated) is the typed side of the same
   surface.
4. **Mixins injections** remain for what no hook point covers; they pin an exact Horizon commit and are the fragile part.

**Verified on 2026-11-08 (roadmap step N2).** Two experimental bundles were built with the official launcher (`horizon-launcher native-build <folder> --server-output <dir> --trust-native-code`) on Horizon nightly `2026-11-07-6560641`: `pluto_lib` (an `[api]` package with a Fabric-like `Event<T>` and `ServerPlayConnectionEvents::JOIN`, fed from the Horizon `player_logged_in` hook) and `tinkers_exp` (`[dependencies] pluto_lib = "^1"`, an `[api]` package whose `hooks()` registers a callback through `pluto_lib::ServerPlayConnectionEvents::JOIN`, with no injection and no Horizon internals). The launcher compiled both (`mod-pluto_lib`, then `mod-tinkers_exp`) into one server engine; the first build took 14 min 36 s. Not run in game: the callback firing on a real join is unverified. Practical notes: the launcher is a Windows GUI binary (start it with output redirected); the version must be installed (`install`) and prepared (`prepare`, about 80 s, reads the owner's Minecraft install) before `native-build` works; `--server-output` builds the dedicated-server engine, the client build goes through an instance.

**Implementation order inside PlutoLib for every API:** (a) a native hook point or the `Hooks` trait if one exists; (b) else a Mixins injection
inside PlutoLib; (c) else a stub plus a support-matrix entry. Mods never see the difference.

Evidence from today's TConstruct: **49 of its 290 injections already sit on native hook points** (32 block behaviour, 11 item use, 5 data/loot,
1 mining); the other 241 inject into internal functions (the whole screen/menu surface, entity events, networking, rendering, block entities,
commands). The first group is cheap and stable; the second is where PlutoLib must absorb upstream churn.

## 3. Packaging, versions and releases (single repository)

- **One repository, several products.** PlutoLib stays in this repository (`pluto-lib/`), as do the mods (`tconstruct/`, later others).
  Each product has its own version and its own release; GitHub Actions builds several files in one run.
- **Tags are per product and named `<directory>-v<semver>`** (decided by the engineering side on the owner's delegation): `pluto-lib-v1.0.0`,
  `tconstruct-v1.1.0`, pre-releases `pluto-lib-v1.1.0-rc.1`. The prefix is the product's directory name, so a tag identifies its product at a glance;
  a hyphen is used instead of a slash so release URLs stay unambiguous and the trigger globs (`pluto-lib-v*`, `tconstruct-v*`) cannot overlap.
  A tag triggers only that product's release workflow, which validates, builds that product's files and publishes them
  (`pluto-lib.hnmod`; `tconstruct.hnmod` + `tconstruct.hmod`). The release check fails unless the tag's version equals the product's `[mod] version`.
- **Each artifact states what it needs:** `pluto-lib.hnmod` declares its Horizon (`hooks: N` where it only uses native hook points, else the
  exact Horizon and Mixins commits) in `horizon-native.json`; `tconstruct.hnmod` declares `pluto_lib = "^1"` in `[dependencies]`.
- **Versioning.** Each product uses semantic versioning. PlutoLib's API is stable within a major version; a Horizon update that PlutoLib can
  absorb is a minor or patch release with no change for dependants. A table in the PlutoLib docs lists which Horizon and Mixins commits (or
  hook API) each PlutoLib release was validated on. The weekly `upstream-drift` workflow reports when the pin is stale.
- The current single `v0.5.0` release workflow (all three files under one tag) is replaced by this scheme when PlutoLib packaging is
  restructured (build order, slice 1).

## 4. Modules (built from what Tinkers uses; scope grows only by need)

A *consumer* is only the first user used to test a module; no module exists for a single feature. Counts: legacy TConstruct injections
the module is expected to replace; "native" = of those, how many already target a native hook point.

| PlutoLib module (Fabric module) | Representative API (Fabric names) | Legacy hooks | Native | Host status |
|---|---|---:|---:|---|
| `lifecycle_events_v1` | `ServerTickEvents`, `ServerLifecycleEvents`, `ServerWorldEvents`, `ServerBlockEntityEvents` | in 4 + 13 | 0 | TBD in step 2 |
| `command_api_v2` | `CommandRegistrationCallback` | in 4 | 0 | TBD |
| `resource_loader_v0` / `loot_api_v2` / `recipe_api_v1` | `ResourceManagerHelper`, reload listeners, `LootTableEvents.MODIFY`, recipe serializers, `ResourceConditions` | 20 | 5 | partly: content overlay and data-pack hook points exist |
| `item_api_v1` | `FabricItem`, `FabricItemSettings`, `ModifyItemAttributeModifiersCallback`, `CustomDamageHandler`, `ItemTooltipCallback`, Tool component rules | 31 + 28 | 11 | `tool-rules` exists |
| `events_interaction_v0` | `UseItemCallback`, `UseBlockCallback`, `UseEntityCallback`, `AttackEntityCallback`, `AttackBlockCallback`, `PlayerBlockBreakEvents` | in 31, 28, 10 | 11 + 1 | TBD |
| `entity_events_v1` | `ServerLivingEntityEvents.ALLOW_DAMAGE / AFTER_DAMAGE / AFTER_DEATH`, `ServerEntityEvents` | in 28 | 0 | guarded damage and equipment seams exist but fail on the pinned Horizon |
| `object_builder_api_v1` | `FabricBlockSettings`, `FabricBlockEntityTypeBuilder`, `FabricEntityTypeBuilder`, `FabricItemGroup` | 40 + 13 | 32 | creative tabs and registration exist |
| `api_lookup_api_v1` / `transfer_api_v1` | `BlockApiLookup`, `Storage<FluidVariant>`, `Storage<ItemVariant>`, `Transaction` | in 40 | | inventory and fluid storage exist as Pluto-specific APIs; to be reshaped |
| `data_attachment_api_v1` | `AttachmentType`, on block entities, chunks, worlds | in 13 | 0 | landing-state and similar exist as private state |
| `networking_api_v1` | `ServerPlayNetworking`, `ClientPlayNetworking`, `PlayerLookup` | 24 | 0 | `client-payload` observation exists |
| `screen_handler_api_v1` | `ScreenHandler`, `ExtendedScreenHandlerType`, `HandledScreens`, `Slot` | 98 | 0 | does not exist |
| `rendering_v1` / `renderer_api_v1` / `model_loading_api_v1` | `HudRenderCallback`, `BuiltinItemRendererRegistry`, `Mesh`/`QuadEmitter`, `ModelLoadingPlugin` | 17 | 0 | rendering primitives exist |
| `biome_api_v1` (structures: no Fabric equivalent) | `BiomeModifications` | 2 | 0 | TBD |

Mod-specific code (3 hooks: book key and click handling, station text input) stays in Tinkers, built on these modules.

Support classes for the matrix: **Implemented** (works on the pinned Horizon), **Stub** (callable, inert), **Blocked by Mixins** (stub today).
Known blocked: `source-host-mining-context`, `source-client-mining-context` (mining speed) and `tinkers-chest-pick-color`, which the
pinned Mixins rejects; they map to `item_api_v1` / `events_interaction_v0` and ship as stubs.

## 5. Build order (reviewed)

Each slice: specify, implement, test, move one existing Tinkers group onto it, run `tools/smoke-local`, update the support matrix.

1. **Foundation and packaging.** `Event<T>`, event phases, `Identifier`, `ActionResult`; the support matrix; PlutoLib restructured as a trait mod
   with an `[api]` package; Tinkers made a trait mod that declares `pluto_lib = "^1"`; per-product tags and release workflows. Nothing
   else is built on a base that cannot be installed and depended on.
2. **Native-hook-point modules first (cheapest, most stable):** `resource_loader_v0` / `loot_api_v2`, `item_api_v1`, `events_interaction_v0`,
   `object_builder_api_v1` (block behaviour). These cover the 49 hooks already on native hook points and need only `hooks: N`.
3. `lifecycle_events_v1`, `command_api_v2`, `data_attachment_api_v1`, `entity_events_v1` (re-review the damage and equipment seams on the
   pinned Horizon).
4. `api_lookup_api_v1` / `transfer_api_v1`, `networking_api_v1`.
5. `screen_handler_api_v1`, `rendering_v1` (largest legacy surface, most upstream-fragile, built last on stable lower modules).

Change from revision 1: slice 1 now includes packaging because the dependency mechanism is `[dependencies]` + `[api]`, and the native-hook-point
modules move ahead of lifecycle because they give the most stable value first.

## 6. Open questions for the owner

1. **Are the module names acceptable** (Rust modules mirroring Fabric artifacts, §1)? They can be refined at each slice.
2. ~~Making Tinkers a trait mod~~ Resolved: the mechanism was built and linked (see §2, "Verified"); slice 1 adopts it. Remaining risk: the callback firing in game.
