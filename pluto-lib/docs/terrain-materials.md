# Prepared native materials for bounded terrain geometry

The material implementation based on `a432ce45`, with the minimal u32 tint
correction, passed the bounded official native CI at `e210b8c1`,
[run 42174754526](https://github.com/sofia-gros/horizon-mods/actions/runs/42174754526).
Source 19, std 30, fluid API 21, actual official plan/12 guards/apply, host
library/binary type checks and two actual native BakedFace tests passed. The
exact package is `b448763e77488d5f7c119d99d675351a8d3244db1a80b3dfd374fa439f039e81`.
The detailed receipt is in `terrain-materials-native-proof.json`.

This does not prove native asset loading/reload, CPU mesh differential or timing,
GPU display, or upward simulation/contact. No TConstruct consumer is activated.
The old CPU/Game experiments and the later Mod-author facade are separate
cutoffs and do not inherit this proof.

## One cache and one provider registry

Legacy `Request`, `Binding`, `Catalog`, `Provider`, `register`, `register_all`
and geometry `build` remain. `AppearanceRequest` adds an explicit native
standalone model, secondary texture, RGB tint and vertical anchor to an existing
group/variant. `prepare_with_materials` resolves it in the same immutable
Assets/RenderAssets/MeshTables owner cache and same revision check as the base
catalog. Legacy `prepare` supplies an empty appearance list.

All request/appearance text is charged to the same 1 MiB preparation budget;
appearance count cannot exceed base binding count. The derived cache has at
most 256 distinct material templates per owner triple, each at most six native
faces. Identical model/texture/tint/anchor combinations share one Arc across the
nine level states. This caps copied template faces at 1,536 per owner, in
addition to the existing bounded base/state maps. It does not claim an exact
allocator byte count. Owner budget and stale/conflicting preparation rules are
unchanged. No model/texture lookup or registry/cache lock is added per cell.

`MaterialProvider` joins the same 256-group startup registry as geometry-only
callbacks. Duplicate ownership across callback kinds fails atomically. Both
kinds freeze on first mesh access. A material callback returns `PaintedBatch`,
which pairs the existing fixed 12-quad geometry with Primary/Secondary choices.
`paint_face` selects matching front/back quads without changing their geometry.
There are no caller-supplied raw atlas layers, alpha/pass flags or native faces.
Existing cell/section output limits still apply. Callback purity, bounded work
and thread-safety remain requirements.

## Actual native material facts, not a layer-number substitution

The secondary model must already exist in `StateModels::standalone`. Preparation
requires one nonempty model with no more than six uniquely directed faces, all
using the resolved secondary sprite. It copies fixed-size native ModelQuad and
BakedFace facts without cloning the native material-name String. Native layer,
opacity, fractional-alpha pass classification and Fusion association come from
that actual prepared model. Missing/invalid models or textures publish an
explicitly unavailable catalog; no fake water/lava material is substituted.

Before any custom vertex is emitted, each selected secondary face must match
the ordinary model's facing, cull direction, tint index, shade override, light,
color, AO override, render type, shade and forced-translucency fields. Its model
AO flag must also match. A mismatch rejects the entire staged section. Only the
sprite-derived layer/alpha/Fusion facts are adopted. For either material, an
opaque non-explicit-pass claim must keep its UVs inside the original material
face's finite proven UV envelope, with no epsilon expansion.

Positions/UVs are still supplied by bounded geometry. The selected ordinary
face's nontexture metadata is retained, then the exact native cull/pane filter
and `Mesher::face` perform light, tint, connected texture, pass and vertex work.
RGB tint multiplies retained baked RGB with integer channel division by 255,
preserving the old alpha bits. White is an exact identity, including None.
Native biome tint is applied afterward by the native face code. This is a
defined composition order, not a claim that all possible multiplication orders
have identical rounding.

No-provider/no-claim paths remain native. A geometry-only callback cannot claim
a state with explicit appearance data; it falls back rather than silently
ignoring the style. A material callback needs a matching prepared appearance.
Missing directional templates, incompatible metadata, invalid geometry or an
output-budget failure trigger the same whole-section native replay. Arbitrary
callback side effects cannot be rolled back and remain forbidden. Replay/cache
behavior still needs the actual CPU fixture.

## Consumer preparation, not automatic content migration

`fluid::content_with_render` keeps all original generated ordinary block files
unchanged and adds one full-UV flow-material model for nondefault display facts.
`render_material_model` returns its exact namespaced model selector. Default
styles return exactly the original content and no extra model. These functions
do not install hooks, register providers or move a world cell.

A consumer must register Definition and RenderDefinition together early enough,
generate/install the returned content, and declare the optional model at the
native `Game::native_standalone_models` boundary. Official 624c calls that boundary
BEFORE the `native_blocks`/content-root path. Therefore the old molten-iron
proposal's native_blocks-only registration is insufficient for this new path.
Do not infer that a later registry snapshot can retroactively add native models.
The new material-aware upper projection derives selectors from the single fluid
registry and resolves them at resource assembly. Fresh matching owners and the
ordinary remeshing lifecycle are still necessary after a permitted late content
change; no already-published GPU mesh is invalidated by this API.

The consumer chooses Primary/Secondary per surface, UVs and geometry policy.
It may use `build_oriented` with the provided anchor for ceiling-attached volume
geometry. Upward simulation/contact semantics, corner smoothing, flow vectors,
fog and camera overlays are not implemented here. Neighbor absence still does
not prove air or loaded-world authority. The old 70-family content is not
silently reinterpreted or activated.

## Verification scope

- Exact source 19, production std-module 30 and real API 21 tests passed.
  The API suite used three exact module selections (10 base, 2 revision,
  9 render-registration tests), without unrelated contact cases.
- Official Horizon 624c / Mixins e8 actual plan, all twelve complete-plan guards,
  apply and host library/binary checks passed. Actual rustc invocations were
  required for Pluto API, renderer, client-library adapter and both host targets.
- The complete actual renderer unit-test target compiled. Its two native
  BakedFace compatibility cases passed, with 1,554 unrelated tests filtered out.
  These are material attribute contracts, not loading or meshing tests.
- Final source/file-set/manifest/Cargo.lock seals and generated native OUT_DIR
  comparisons passed. Minimum free disk was 87,804,133,376 bytes.
- Four earlier failed attempts remain recorded: one tint integer compile error
  and three strict selector/name validation failures. Their partial passing
  tests are not presented as complete-run successes. Runtime was unchanged
  during the three selector repairs.
- The legacy modeled binding fixture still rejects this material-aware source.
  No fake positive catalog replaces native resources. Live loader/reload,
  section fallback/cache replay, CPU performance and GPU checks remain open.
