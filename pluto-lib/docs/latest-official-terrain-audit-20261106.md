# Latest official terrain compatibility: source audit

This is a selected-source audit, not a new planner, type, CPU, GPU or live-game
result. No production source or guard was changed. The companion JSON records
23 exact Horizon files, seven Mixins files, historical guard inputs and URLs.
All comparisons use fixed-revision file bytes. The GitHub compare listing is
truncated at 300 files for this Horizon range and is not a complete change list.

## Cutoffs stay separate

| Purpose | Horizon | Mixins |
| --- | --- | --- |
| Historical common and CPU evidence | `624c4f6265c02296fbe8a80f029dd0d987fb94c0` | `e8cd0aa38cffef49cb2e088ef716baa107148c02` |
| New source audit and future diagnostic | `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd` | `5a64af938ec86b00d53d16a408aabc0038fb3e57` |

Historical runtime is `43f17427`, successful CPU fixture `1ac143d8`, and its
closure `2c78ab51`. The separate profile source `b0fc4960` has not executed.
Historical common bundle `4f13a6e2` is preserved byte-for-byte. None of these
results is relabeled as latest compatibility. A two-manifest revision-only
plan diagnostic is separate from a deployable package or a compatibility fix.

## Native rendering seam

- The complete 89,342-byte
  [mesher.rs](https://github.com/horizon-rust/horizon/blob/f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd/crates/horizon-client-render/src/mesher.rs)
  is unchanged: SHA256 `364bdbb89839984eaa9819b75170a3ccbe5c73476aec965835edd4eb7e280c19`.
  This includes the copied `mesh_reusing` body and original face/cull/vertex
  code. There is no source reason here to invent new dispatch hunks.
- Native fluid meshing, baked catalog, asset model decoder/baker, sprites,
  Fusion rules/layout/texture, model library, and FaceVisibility source are
  also unchanged in the inspected files. This preserves the source premise
  of the supported `shade_direction_override` negative fixture; it does not
  execute that fixture on latest dependencies.
- The complete `PackResources` impl and its `assemble` body are unchanged.
  The [latest assembly](https://github.com/horizon-rust/horizon/blob/f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd/crates/horizon-client/src/game.rs#L2277)
  still receives the same retained Assets/RenderAssets/MeshTables owners.
  Its containing file changed, so old source locations and module
  fingerprints must be regenerated from an actual new plan, not guessed.
- The worker's mesh-job/WorkerContext span is byte-identical. Other
  [streaming changes](https://github.com/horizon-rust/horizon/blob/f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd/crates/horizon-client-render/src/streaming.rs#L1172)
  add `relight_resent` and distinguish Horizon bulk-edit relighting from
  other servers' packet light. Job accounting also omits clock reads for
  the short non-mesh job category. A selected CPU section fixture cannot
  establish complete streaming or multiplayer behavior.

## Loader, registration and reload changes

The [startup order](https://github.com/horizon-rust/horizon/blob/f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd/crates/horizon-client/src/game.rs#L22702)
still registers standalone models before `content_root` and resource loading.
A native entity-layer registration call is added after standalone registration.
The fluid author's startup contract therefore still has a source basis; actual
consumer registration/activation is not established by this audit.

The new [live Mod switch path](https://github.com/horizon-rust/horizon/blob/f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd/crates/horizon-client/src/game/mods.rs#L476)
tracks applied content separately from desired switches. Asset/data-only Mods
can trigger resource reload and then server data reload; blocks/items/worldgen
still require restart. `packs.rs` uses applied Mods and calls `mods_reloaded`
after selection reload. New cutoff testing must cover new resource-owner
triples and fail-closed unavailable/stale preparation through this path. The
source change alone neither proves a stale cache bug nor proves safe reload.

Assets, models, biomes and BlockFacts loaders now use
[json_file](https://github.com/horizon-rust/horizon/blob/f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd/crates/horizon-world/src/json_file.rs),
which retains serde_json parsing while adding the file path to errors. This is
not lenient parsing. BlockFacts also returns air's facts for an unknown state
instead of direct indexing. Existing synthetic valid-state fixtures do not
prove this new edge case.

`Assets::reload_with_models` still directly calls `load_with_parsed`, bypassing
`RenderAssets::load_with`; the [actual caller](https://github.com/horizon-rust/horizon/blob/f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd/crates/horizon-client/src/game.rs#L21033)
remains. The earlier main slime-colormap HEAD coverage concern therefore
remains distinct from Pluto's resource-assembly adapter. Do not discard or
silently replace that consumer behavior.

## Existing guards must continue to reject drift

The historical cell guard has seven official source pins: only `streaming.rs`
changed. The binding guard has ten: `game.rs`, `game/mods.rs`, `game/packs.rs`,
`assets.rs`, and `streaming.rs` changed. Their old hashes correctly reject these
inputs. Geometry metadata also names old 624c. No pin was loosened or updated.

On a reviewed new cutoff, retain the full copied-body and face-filter inverse,
exact module/helper identity and whole-mesher-file injection exclusion. Obtain
actual new plan nodes and fingerprints before recording them. Do not erase the
old checks or accept a hash mismatch merely because selected methods match.
Source equality does not establish the new dependency or compiler graph.

The new Horizon lock SHA256 is
`0bc4234f3001d44df9d46fbb06df331d3f53b1af6b613ec9082baaec3d93c62f`.
The root workspace adds `horizon-mod-deps`. Fresh lock/path/compiler witnesses
are needed; cached old CPU output is not evidence for the new cutoff.

## What latest Mixins actually changes

The two intervening commits add crate self-alias target lookup and limited
workspace-glob analysis. In
[index.rs](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/index.rs#L546),
exact target matches win; only absent direct matches fall back through
`extern crate self as alias`. Existing ambiguity rejection remains. Workspace
root canonicalization is also explicit.

In [modules.rs](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/modules.rs#L102),
the planner follows a top-level glob only when its destination resolves to one
indexed source-file identity. It checks the proposed name against declarations,
imports, nested globs and already planned modules there. Unknown/external/inline
or ambiguous destinations, excessive recursion, and actual name collisions
still reject. Source tests cover a workspace re-export with no collision and
direct/transitive collisions, but those tests were not run in this audit.

The transformer verifies original fingerprints and trusts the planner's glob
analysis; this is not permission to skip planning. Payload file attributes,
external modules/includes, visibility/dependency rules, and Pluto's own mesher
exclusion are not relaxed. Current mesher support placement does not need
relocation simply because other workspace roots can now accept some modules.
Whether exact main/common composition advances past earlier failures remains
an actual-plan question owned by the separate diagnostic.

## New cutoff sequence

1. Preserve the immutable original bundles and run the authorized revision-only
   main/common plan diagnostic once. Record its first actual failure and any
   unchanged guard failure; do not call revision editing a repaired package.
2. For a separate terrain upgrade candidate, review the changed loader/reload
   dependencies and actual new plan identities. Keep exclusivity/inverse guards;
   update only reviewed cutoff metadata and compatibility manifests together.
3. Run ordinary complete apply/type gates on latest, then the exact real-type
   synthetic CPU oracle with fallback/replay and supported material assertions.
   Keep production assets, GPU and live gameplay outside that evidence.
4. Only then run the separately frozen profile extension with explicit compiler
   profiles and fresh timing processes. Preserve full end-to-end totals, paired
   same-output assertions and callback/kernel measurements. Do not compare old
   full-height native geometry against half-height claims as pure overhead.

No new native or remote job is started by this artifact. The Mod-facing thin
facade and its internal registry remain unchanged; a compatibility audit does
not introduce a second registry or migrate TConstruct policy.

## Reproduction

`tools/audit_latest_terrain.py` accepts explicit old/new Horizon and Mixins
source directories, the historical common tools directory and archive. It
only reads and hashes files with Python. It checks all 30 selected sources,
the three native spans, historical guard changes, archive identity and the
source-only claim boundary. No compiler, metadata, Rust formatter or planner
is invoked. The recorded result is six passing bounded audit groups; it is not
a count of native tests.
