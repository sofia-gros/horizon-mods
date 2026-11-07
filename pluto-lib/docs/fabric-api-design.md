# PlutoLib specification (draft for owner approval)

Roadmap step 1. Principles: [design-principles.md](design-principles.md). Evidence: [fabric-api-hook-map.md](fabric-api-hook-map.md)
(the 290 legacy TConstruct injections mapped to Fabric API modules by `tools/map_hooks_to_fabric_api.py`; the mapping is
by host function and injection id, a heuristic to be reviewed, not a final design).

Status: **draft**. Nothing here is implemented. It becomes binding when the owner approves it.

## 1. Conventions

**Mimic Fabric API.** Same module boundaries, type names, constants and call shapes, translated only where Rust requires it.

| Fabric API (Java) | PlutoLib (Rust) |
|---|---|
| artifact `fabric-lifecycle-events-v1` | crate `pluto-lifecycle-events-v1` |
| package `net.fabricmc.fabric.api.event.lifecycle.v1` | module `pluto_api::event::lifecycle::v1` |
| `ServerTickEvents.END_SERVER_TICK.register(cb)` | `ServerTickEvents::END_SERVER_TICK.register(cb)` |
| `Event<T>` with `register`, phases, `EventFactory` | `Event<T>` with `register`, `register_with_phase`, `EventFactory` |
| `ActionResult` returns from callbacks | `ActionResult` (same variants) |
| `Identifier` (`namespace:path`) | `Identifier` |
| class `FabricBlockSettings`, `FabricItemSettings` | struct `FabricBlockSettings`, `FabricItemSettings` (builder methods snake_case) |

Rules: types and constants keep the Fabric spelling (`UseItemCallback`, `ALLOW_DAMAGE`); functions and fields are snake_case;
module paths mirror Java packages; semantics (event ordering, cancel results, side: server or client) follow the Fabric
documentation. A deliberate difference from Fabric is recorded next to the API, with the reason.

**Always the full API.** Where the pinned Horizon or Horizon Mixins cannot do something yet, the call exists and its body is a stub
(a no-op, `false` or the documented default). A *support matrix* (`pluto-lib/docs/support-matrix.md`, to be created with the first
module) lists every stub, why, and which host change unblocks it.

**Packaging.** PlutoLib is an independent mod (its own `.hnmod`, plus content `.hmod` when needed) installed beside the mods that
use it. A mod declares a dependency on PlutoLib in its manifest. Mods ship no Horizon Mixins injections of their own.

**Versioning.** One PlutoLib release per Horizon revision, `MAJOR.MINOR.PATCH+horizon.<revision>`; a compatibility table lists the
Horizon and Mixins revisions each release was validated against. Upstream moves quickly, so a release tracks a *pinned* revision and the
weekly `upstream-drift` workflow reports when the pin is stale. Absorbing upstream change is PlutoLib's job.

## 2. Modules (built from what Tinkers uses; scope grows only by need)

Counts are legacy TConstruct injections the module is expected to replace (from the hook map). A *consumer* is only the first user used
to test a module; no module exists for a single feature.

| PlutoLib crate (Fabric module) | Representative API (Fabric names) | Legacy hooks | Host status |
|---|---|---:|---|
| `pluto-lifecycle-events-v1` | `ServerTickEvents`, `ServerLifecycleEvents`, `ServerWorldEvents`, `ServerBlockEntityEvents` | in 4+13 | TBD in step 2 |
| `pluto-command-api-v2` | `CommandRegistrationCallback` | in 4 | TBD |
| `pluto-resource-loader-v0` / `pluto-loot-api-v2` / `pluto-recipe-api-v1` | `ResourceManagerHelper`, reload listeners, `LootTableEvents.MODIFY`, recipe serializers, `ResourceConditions` | 20 | partly exists (content overlay, data pack hooks) |
| `pluto-item-api-v1` | `FabricItem`, `FabricItemSettings`, `ModifyItemAttributeModifiersCallback`, `CustomDamageHandler`, `ItemTooltipCallback`, Tool component rules | 31 + 28 | `tool-rules` exists |
| `pluto-events-interaction-v0` | `UseItemCallback`, `UseBlockCallback`, `UseEntityCallback`, `AttackEntityCallback`, `AttackBlockCallback`, `PlayerBlockBreakEvents` | in 31, 28, 10 | TBD |
| `pluto-entity-events-v1` | `ServerLivingEntityEvents.ALLOW_DAMAGE / AFTER_DAMAGE / AFTER_DEATH`, `ServerEntityEvents` | in 28 | guarded seams exist (damage, equipment) but fail on the pinned Horizon |
| `pluto-object-builder-api-v1` | `FabricBlockSettings`, `FabricBlockEntityTypeBuilder`, `FabricEntityTypeBuilder`, `FabricItemGroup` (creative tabs) | 40 + 13 | creative tabs and registry exist |
| `pluto-api-lookup-api-v1` / `pluto-transfer-api-v1` | `BlockApiLookup`, `Storage<FluidVariant>`, `Storage<ItemVariant>`, `Transaction` | in 40 | inventory, fluid storage exist as Pluto-specific APIs; to be reshaped |
| `pluto-data-attachment-api-v1` | `AttachmentType`, attachments on block entities, chunks, worlds | in 13 | landing-state and similar exist as private state |
| `pluto-networking-api-v1` | `ServerPlayNetworking`, `ClientPlayNetworking`, `PlayerLookup` | 24 | `client-payload` observation exists |
| `pluto-screen-handler-api-v1` | `ScreenHandler`, `ExtendedScreenHandlerType`, `HandledScreens`, `Slot` | 98 | does not exist |
| `pluto-rendering-v1` / `pluto-renderer-api-v1` / `pluto-model-loading-api-v1` | `HudRenderCallback`, `BuiltinItemRendererRegistry`, `Mesh`/`QuadEmitter`, `ModelLoadingPlugin` | 17 | rendering primitives exist |
| `pluto-biome-api-v1` | `BiomeModifications` (structures and features: no Fabric equivalent, NeoForge-style) | 2 | TBD |

Mod-specific code (3 hooks: book key and click handling, station text input) stays in Tinkers, built on the modules above.

Support classes for the matrix: **Implemented** (works on the pinned Horizon), **Stub** (callable, inert), **Blocked by Mixins** (stub today; the
three TConstruct injections the pinned Mixins rejects: `source-host-mining-context`, `source-client-mining-context`, `tinkers-chest-pick-color`,
map to `pluto-item-api-v1` / `pluto-events-interaction-v0`).

## 3. First build order (slices follow modules)

Each slice: specify, implement, give it tests, move one existing Tinkers group onto it, run `tools/smoke-local`, update the support matrix.

1. Foundation: `Event<T>`, `EventFactory`, `Identifier`, `ActionResult`; the support matrix; the packaging and version scheme. (No hook yet; testable.)
2. `pluto-lifecycle-events-v1`, `pluto-command-api-v2`, `pluto-resource-loader-v0` (smallest legacy surface, lowest host risk).
3. `pluto-item-api-v1` and `pluto-events-interaction-v0` (use, mining, tooltips; includes the stubs for the Mixins-rejected hooks).
4. `pluto-entity-events-v1` (re-review the guarded damage and equipment seams on the pinned Horizon).
5. `pluto-object-builder-api-v1`, `pluto-api-lookup-api-v1`, `pluto-data-attachment-api-v1`.
6. `pluto-networking-api-v1`, `pluto-screen-handler-api-v1`, `pluto-rendering-v1` (largest legacy surface, built last on stable lower modules).

## 4. Open questions for the owner

1. **Naming of the Rust paths** (`pluto_api::event::lifecycle::v1`) and crate names above: approve, or pick a different mirror of the Java packages.
2. **Repository layout:** keep `pluto-lib/` in this repository with its own version and release tag, or split into its own repository now?
3. **Manifest:** how does a Horizon mod declare its dependency on another mod today (the `.hnmod`/`.hmod` manifests)? PlutoLib's dependency declaration should reuse that mechanism.
4. **Build order** above: approve or reorder (the order only affects sequencing; module scope follows Fabric API).
