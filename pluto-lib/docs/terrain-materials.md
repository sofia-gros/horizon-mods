# Prepared native materials for bounded terrain geometry

This follow-up is a SOURCE WIP on the first render-contract checkpoint
`a6e25c8e` (published `52c551d1`). It has not passed a new official native
plan/apply/type gate, CPU differential, timing run or GPU display test. The old
frozen CPU/Game CI checkpoints are unchanged; their results cannot validate
this different source. No TConstruct consumer is activated.

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

- Nineteen terrain Python/source/package checks and exact copied native body /
  face-filter regeneration passed against official 624c source.
- Seven new pure Rust tests cover appearance validation, shared callback kinds,
  fixed material selection, tint and additive content generation. Two additional
  native-type tests cover compatible versus mismatched BakedFace attributes.
  They are written but unexecuted; earlier first-cutoff tests are also pending.
- The legacy modeled-renderer binding fixture now rejects this source before
  creating output. It lacks actual native material templates and must not be
  expanded with a fake positive model catalog. Full native material proof needs
  the real renderer types/resources in the next authorized fixture.
- No native compiler, Cargo/metadata, runtime renderer or GPU run is claimed.
