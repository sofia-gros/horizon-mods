# Slime islands (in progress)

The source definitions and island templates in `content/reference` are pinned
to TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. The original
Overworld sky-island set has spacing 35, separation 4, frequency 0.5 and
weights 4:1 for sky slime and clay islands. Sea and Nether islands and End sky
islands use their own original structure sets and biome tags.

`source-slime-island-placement.rs` implements the original sea height and sky
height calculation, including the rotation-dependent offsets and the four
surface samples. It is tested as a standalone rule and is not connected to
world generation. The 20 original earth, sky, ender and blood island NBT
templates are retained without replacing vanilla structures.

`source-slime-tree-shape.rs` models source tree height, slimy-soil/build-bound
checks, the ordered diamond canopy, air removals, leaf drippers and shared
vine-state groups. It is also standalone: randomized vine states, world soil
queries, block writes and leaf-distance updates are not connected yet.
`source-slime-island-markers.rs` models source weighted selection and the
per-piece limit of three successful trees, including skipped draws and failed
placements. Persisting that count and invoking it from marker processing are
still open.

The original uses `tconstruct:island`, not a vanilla jigsaw: its templates
include data markers that can place source slime trees, vines and grass. The
current Horizon structure registry recognizes the type as unsupported.
Tree/foliage registration, original drops and survival acquisition, marker
processing, worldgen integration, save/LAN and real-world validation remain
open. Sky/End slime mob spawn overrides also require their own entity support.
No island or slime-plant natural generation is claimed by this draft.

The existing `TemplatePiece::place_at` can return original data markers, but
its `PieceContext` exposes `dyn PieceWorld` rather than the configured-feature
callback needed for a marker's tree. An official Mixins limited plan for
`horizon_world::worldgen::structure::start::place_in_chunk` reports
`E_UNSUPPORTED_SIGNATURE` because that function is generic. A separate plan
for the `place_in_chunk` call in `generator_decoration::Decoration::decorate`
reports `E_UNSUPPORTED_CONTROL_FLOW` because the call is inside a closure.
Limited cancellable HEAD probes for `try_generate` and
`StructureState::use_saved_start`, with their registry, context, position and
saved-piece bindings, both passed transformation and `horizon-world` type
checking. They are no-op probes only: no source piece, serialization or
marker connection is installed, and they do not prove save/reload.
An adapter over `dyn PieceWorld` type-checks for basic block and height
access, but the piece context does not expose the host's named-feature
catalog or complete tree state-provider bindings. The retained original
`tconstruct:slime_tree` definition currently returns `Field("basic feature
type")` from the host feature decoder. A vanilla tree is not a substitute
for the source algorithm and is not installed here.
