# Custom-fluid consumer migration boundary

This is a source-only audit, not a new API, activated provider or recovered TConstruct feature. TConstruct uses PlutoLib; other Mixins mods need not. Official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02` are the only host inputs.

## Exact source scopes

- Current shared foundation: local `8715b1232c0fca133fe012f35830e2108e653d59`, equivalent published v15 tree at `89744814b8d536a117e82f8ea3c8cdc9798faa95`. Its ordinary installed plan/apply/type proof is separate from the unexecuted CPU renderer fixture.
- Legacy TConstruct content: [PR #932 head d82b7664](https://github.com/sofia-gros/horizon-mods/tree/d82b7664cfda55a8a8e850e97808c77dcdbcb732/tconstruct/content/data/tconstruct/registered_fluid). Exactly 70 JSON definitions, each with a distinct source ID. This is the old `registered_fluid` schema, not a current Pluto consumer.
- Molten iron proposal: local `77a3b67ab4c6c38e02d306481ab8d6afbd6862f9`, `tconstruct/content/reference/world-fluids/molten-iron.json` and `world-fluid-molten-iron-registration.rs`. It is an unintegrated experimental slice, not authoritative official-host operation.
- Companion JSON records source hashes, exact counts and family names. No discarded host fork was read.

The current TConstruct tree has no matches for `pluto_api::fluid`, `register_world_fluid` or `pluto_terrain_cells::register`. The shared renderer explicitly registers no consumer. Thus shared type-check success does not establish that the old 70 families are rendering through this path.

## Concrete migration gaps

1. **Capacity is smaller than the legacy family set.** `terrain-cell-providers.rs` permits 64 exact-group providers. `fluid-render-prepare.rs` uses `Definition.id` as the group; the old set has 70 distinct source IDs. A migration that assigns a callback to every family would hit `Limit` at the 65th registration. Reusing the same function pointer does not share a registry slot. This is a future full-family admission gap, not a present active-consumer regression. No bound was changed by this audit.
2. **One sprite cannot preserve the old visual contract.** Every legacy definition names distinct still and flowing textures; 12 also specify a nonwhite tint. Current `Definition` has one `texture` and `light`; the prepared binding has one sprite. Eligible selected models must use that same sprite on every face. Geometry providers change only positions/UVs and retain native baked face tint/material/light metadata. Merely copying legacy IDs into the new schema cannot preserve the separate textures or automatically install the old tint policy. A future visual translation needs an explicit scope and real resource fixtures, not a silent still-texture substitution.
3. **Old content is not an eligible ordinary model.** Legacy molten iron uses a water-template block, a single default model and `elements: []`. The new adapter requires a nonempty selected ordinary model, at most six unambiguous directional faces, and native `FluidKind::Empty`. Its catalog asks for nine explicit `level` states. The proposed molten-iron slice instead generates glass-template, noncolliding ordinary blocks and nine nonempty models. Merely registering the old files cannot satisfy the empty-model check, irrespective of how a host resolves the template. New content and any saved-world conversion need their own proof; this audit does not perform either.
4. **Neighbor information is intentionally incomplete.** The provider sees only prepared bindings in a 3x3x3 render neighborhood. `None` does not distinguish air, solid, an unregistered fluid or an unloaded neighbor; native padding can repeat chunk edges. Do not implement fluid occlusion, source smoothing or edge behavior by assuming `None` is air. A bounded consumer may return `None` to preserve its native model when facts are insufficient.
5. **Upward fluid behavior is not established.** Legacy `ichor` and `molten_cinderslime` explicitly flow upward. Current flow/content conventions are downward level 0 source, 1–7 horizontal and 8 falling; the geometry builder describes a bottom plane beneath four top heights. General inverted-volume semantics are not promised. Do not label those two families recovered by downward geometry alone. Flow/contact changes remain outside this rendering audit.

## Registration and reload contract to preserve

The official `crates/horizon-client/src/game/mods.rs` `native_declared` uses one process-wide `OnceLock` and invokes `Game::native_blocks` before content loading. That is the proposed slice's registration boundary. `PackResources::assemble` receives the actual retained Assets/RenderAssets/MeshTables owners; the shared HEAD prepares the derived catalog there. Source ordering is useful evidence, not a live startup/reload test.

The authoritative definition registry and renderer provider registry have different lifetimes. Successful definition additions invalidate the old snapshot; unchanged/failed additions do not. Providers freeze at the first `mesh_reusing`, even when initially empty, and late registration is rejected. Definitions added after preparation require a new resource preparation and ordinary remeshing lifecycle; an already frozen provider map cannot gain a new family. Existing GPU meshes are not automatically evicted by snapshot invalidation. A consumer must register both bounded sets before initial meshing and state its reload policy explicitly.

Missing states/textures, conflicting state bindings, stale revisions or mismatched live owner triples do not authorize native-water/lava coercion. The catalog is unavailable and the adapter keeps ordinary native behavior. A revision post-check discards only staged return data; providers must be pure, and arbitrary callback side effects are not rolled back.

## Next honest consumer gate

Before modifying a consumer, choose a concrete bounded family subset and specify whether it promises only stepped ordinary-block geometry or the complete old visual policy. Preserve source formulas in TConstruct and mechanical rendering services in PlutoLib. Verify real generated state/model/texture compatibility, early provider registration, absent-neighbor behavior and reload rejection. Resolve the 70-versus-64 admission issue only as part of a reviewed full-family requirement.

The already published manual CI checkpoints remain unchanged. CPU fallback/differential and timing evidence is still pending; a passing synthetic real-type CPU test would not by itself prove production textures, GPU display, full-family rendering, upward flow, fog, camera overlays, currents or multiplayer behavior. This audit ran only source/JSON inspection and introduced no runtime code or feature activation.
