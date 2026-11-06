# Main/common terrain composition and CPU cost audit

This is a source-only audit, not a combined Mixins plan, Rust check or rendering
result. No production code, compatibility pin, manifest, existing guard or
consumer behavior is changed by this checkpoint.

## Exact inputs and evidence

- TConstruct main: [`08afa992d5091e03360f3473ea93fd17aa793987`](https://github.com/sofia-gros/horizon-mods/commit/08afa992d5091e03360f3473ea93fd17aa793987).
- Common source: [`035273c4805d75111e528c8c26607fa3a8d1d5bf`](https://github.com/sofia-gros/horizon-mods/commit/035273c4805d75111e528c8c26607fa3a8d1d5bf), successful independent common run [42189798838](https://github.com/sofia-gros/horizon-mods/actions/runs/42189798838).
- Actual common `.hnmod`: SHA-256 `4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d`, 270,125 bytes, 147 ZIP members, **one** installed manifest: 34 injections, 69 modules, 3 states, 2 crates. Excluded component manifests in the source tree were not counted as installed.
- Official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`, Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`.
- Main still declares Horizon `0c7b5b61a5e746d621afffba919b12aeccf9c0fa` and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. **This mismatch remains unresolved.** No declaration was silently rewritten and no planner outcome is predicted.

`audit_main_common_terrain.py` performed nine bounded Python source/policy checks
and recorded 36 input hashes in the companion JSON. Three of those checks use
synthetic dictionaries to exercise the exact Python guard's selection logic.
Those dictionaries are deliberately not represented as real Mixins output.
The seven official inputs listed by the terrain guard matched their source
hashes; regeneration/rustfmt, planner, apply, Cargo and Rust were not invoked.

## Four modules are not four active face hooks

The [main manifest](https://github.com/sofia-gros/horizon-mods/blob/08afa992d5091e03360f3473ea93fd17aa793987/tconstruct/mixins/tconstruct/horizon-mixin.toml)
adds these support modules to `horizon-client-render/src/mesher.rs`:

| Module ID | Source | Actual current connection |
| --- | --- | --- |
| `storage-render-info-mesher` | `source-render-info.rs` | Material metadata/sprite selection. A second copy in `streaming.rs` supports the active region metadata hook. Mesher face consumer is disabled. |
| `connected-texture` | `connected-texture.rs` | TConstruct family, UV-direction and cornerless-suffix rules. Face consumer is disabled. |
| `chest-tint-math` | `source-chest-tint-math.rs` | Packed vertex RGB multiplication retaining the high byte. Face consumer is disabled. |
| `source-slime-colormap` | `source-slime-colormap.rs` | Active asset-load hook populates the map; the face tint helper is not registered. |

There are **zero active injections targeting `::mesher::`** in this main
manifest. The separate [disabled manifest](https://github.com/sofia-gros/horizon-mods/blob/08afa992d5091e03360f3473ea93fd17aa793987/tconstruct/disabled-injections.toml)
holds `storage-mesh-face` and `connected-mesh-face` (cancellable HEAD on
`Mesher::face`) and `tinkers-chest-world-tint` (RETURN on the same function).
The native packager reads the active manifest and does not import that separate
file. Merely packaging dormant `.rs` files does not activate them.
`source-slime-colormap-face.rs` is referenced by neither manifest.

The common [terrain guard](https://github.com/sofia-gros/horizon-mods/blob/035273c4805d75111e528c8c26607fa3a8d1d5bf/pluto-lib/tools/check_terrain_cell_plan.py)
reserves **injections** in the entire mesher source file, plus aliases matching
its target/fingerprint. It requires the one reviewed Pluto `mesh_reusing` HEAD.
It verifies its five modules but does not reject every additional support
module. The current four TConstruct declarations have distinct names. They
alone therefore do not trigger this particular exclusivity rule. Other
planner/module/type/complete-plan failures remain possible.

Conversely, restoring any of the three disabled face injections would add an
injection in the reserved file and is rejected by the present guard. Do not
solve this by deleting the guard, allowing arbitrary face hooks, silently
dropping the old features or calling their helpers directly from another mod.
The remapped Pluto face deliberately has an empty material sprite string;
legacy code that inspects that string is not automatically composable.

## Active neighboring seams and ordering

1. **Atlas setup:** `storage-atlas` intercepts `Sprites::insert` at the destroy
   stage 9 sentinel. It adds storage fallback and connected-glass suffix sprites
   before resource assembly. It does not select those sprites for a face.
   Startup failure/order and native atlas compatibility still need actual
   combined proof; Pluto does not add a second hook at this target.
2. **Resource assembly:** common has an exclusive noncancelling HEAD on
   `PackResources::assemble`, after assets/render/tables exist. Its catalog is
   bound to the exact three live owners and registry revision. Main has no
   hook on `PackResources::assemble`. Appending a second preparation hook there
   is not a proposed workaround.
3. **Region metadata:** `storage-render-init` loads metadata at `prepare_lod`;
   `storage-region` can replace `Streaming::region` with the same 3x3 chunk
   collection and a COW center saved-root metadata addition. Native and copied
   mesh bodies consume that region later. This phase is outside the mesher-file
   injection selector, but that does not prove its full compatibility or
   freshness. Current cell callbacks cannot read arbitrary saved block entities.
4. **Face visibility:** active `connected-pane-face` modifies
   `FaceVisibility::skips`. Both the native body and copied body call
   `should_render_face`, which calls `skips`, before per-cell emission. This
   lower-level hook is not bypassed merely because the mesher body is copied.
   The main pane marker/neighbor behavior still needs combined tests. Existing
   synthetic CPU evidence did not install this TConstruct hook.
5. **Slime map lifetime:** active `source-slime-colormap-load` hooks only
   `RenderAssets::load_with`. Official `Assets::reload_with_models` directly
   calls `load_with_parsed`, bypassing `load_with`; `game.rs:20543` uses that
   route when parsed models are available. The map hook therefore does not
   cover every inspected native resource-load route. Its process-global map
   is not an owner-bound prepared render fact. This is a proven call-chain
   coverage gap, **not evidence that a stale visual was observed**. Independent
   read-only review confirmed the route. Do not erase this feature or claim
   that moving a hook alone proves resource-lifetime correctness.

Official source links: [asset routes](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client-render/src/assets.rs#L289-L322),
[native call site](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game.rs#L20529-L20547),
[visibility](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-world/src/block_render.rs#L157-L221),
[face path](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client-render/src/mesher.rs#L1644-L1890).

## Minimum migration units, without weakening the guard

**First establish composition with no feature edits.** Preserve the four support
modules, related active hooks and all disabled source definitions. The current
pin mismatch and actual complete-plan diagnostics are independent gates; this
audit neither resolves them nor licenses filtering failed hooks from a proof.

**Then restore features individually through a Pluto-owned boundary:**

- Connected glass needs original baked-face selection, face-local UV directions,
  neighboring block/pane-arm facts, prepared suffix textures and exact tint
  semantics. Existing fluid cell callbacks are insufficient for multipart panes
  or arbitrary original baked quads. Prefer first investigating equivalent
  native Fusion/connected-texture resource rules. If those cannot express the
  predicates, design a bounded prepared face-material selector inside Pluto's
  reviewed adapter, retaining the complete native light/Fusion/overlay/routing
  path. Keep TConstruct family IDs and connection policy in TConstruct.
- Material storage/anvils need per-region/per-block-entity material facts,
  variant/fallback resolution, distinct worktable-bottom selection, tint and
  luminosity. Preserve the existing metadata path until an immutable
  region-scoped fact input and original-face material boundary are proven.
  The current `CellContext` does not provide those facts; pretending the fluid
  group registry does would introduce the wrong ownership and geometry model.
- Modifier chest tint is a separate contract. Its old RETURN edits the last
  four output vertices. Native faces can emit quarters/overlays/multiple passes,
  so that cannot be restored unchanged. A pre-emission tint contribution needs
  an explicit parity decision: moving packed RGB multiplication before native
  shading can change integer rounding, overlay tint and high-byte meaning.
  Establish that contract and exact oracle before enabling it.
- Slime foliage tint needs an owner-bound prepared colormap plus current
  position/state facts at the native tint phase. Current `Appearance.tint` is
  constant per prepared group/variant and cannot represent its coordinate
  function. Preserve color policy; do not add a second authoritative registry
  or a process-global cross-world render cache.

Any later generic boundary changes are a **new adapter cutoff** with new
reviewed regeneration hunks, unchanged exclusive whole-plan policy and actual
composition/type/CPU tests. They are not approved or implemented by this memo.

## CPU cost: measured result versus source hypotheses

The accepted [CPU proof](https://github.com/sofia-gros/horizon-mods/commit/2c78ab519fc7411bc3ff83aaa0ce1616f4c0b36f)
is exact run [42192378071](https://github.com/sofia-gros/horizon-mods/actions/runs/42192378071),
source `1ac143d840eaf3dde9e108ada3f5f2ec9f4c451f`. It uses original synthetic
resources, **opt-level 0**, debug assertions and 256 separated cells per section.
Median adapter/native ratios: no provider 1.0064, registered ordinary 1.0092,
provider returns None 1.0321, claimed geometry **1.8855**, secondary material
**1.9265**. These are not optimized game/FPS numbers or optimization wins.

Both claimed providers emit six front faces with `back=false`; the roughly
twofold cost is not explained by simply emitting twice the number of faces.
However, native full-height versus claimed half-height geometry is different,
and the timed geometry callback also performs assertion/neighborhood queries.
The result measures that whole workload, not isolated Pluto dispatch overhead.

Source-only candidate costs:

- Each claimed cell builds/initializes a fixed 12-quad batch, validates geometry,
  then the adapter validates cell-local bounds again. This is allocation-free
  but not free under opt-level 0. Private builder bounds and safe origin checking
  must not be removed merely because one fixture uses origin zero.
- Per-cell state/group/material map lookups, selected-face scans, per-quad
  material compatibility/UV proof, remapping and native emission add work.
  There is no per-cell mutex acquisition or asset loading in this path. The
  owner-cache lock/weak-owner lookup and revision checks occur per section.
- `opacity_allows` arguments currently construct the target UV array even when
  the native face is nonopaque or has an explicit pass and no subset calculation
  is needed. A narrow later short-circuit can avoid that redundant copy while
  retaining the same proof for opaque implicit-pass faces. Benefit is unmeasured.
- Immutable baked compatibility/UV-envelope facts and frozen group dispatch may
  be preparation candidates, but any cache must remain bound to actual live
  resource owners and selected models. Do not key a new global cache by bare
  pointers, assume a single model for all states, move callbacks ahead of their
  current eligibility checks or cache outputs of world-dependent callbacks.

Recommended next measurement: retain this exact oracle/receipt; add an optimized
profile as a separately identified gate and measure callback construction,
validation and emission independently with equal output counts. Keep the current
correctness assertions in the correctness test, while a separately proven timed
callback can omit test-only assertions. Benchmark one semantics-preserving
candidate at a time. Maintain ordinary/no-claim, opaque UV, animated/Fusion,
invalid/stale whole-output fallback and scratch replay cases. No such new run
or production optimization was performed here; GPU/live-game/Tinkers activation
remain outside the successful CPU proof.

## Reproduce this audit only

Run `pluto-lib/tools/audit_main_common_terrain.py` with `--repo` pointing to a
repository containing exact main, `--common-source` to exact common source,
`--bundle` to the fixed 4f13 archive and `--official-source` to the pristine 624c
export. Set `PYTHONDONTWRITEBYTECODE=1`. It reads inputs, invokes only `git show`
and Python, and emits the report to stdout. It does not fetch repositories,
modify these inputs, execute native tooling or start Actions.
