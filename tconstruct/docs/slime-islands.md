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
vine-state groups. `source-slime-tree-world.rs` uses the structure piece
world interface for soil checks, randomized vine states, tree block writes
and source leaf-distance updates. A small in-memory world exercises soil
rejection and basic tree writes; the function has not been connected to a
structure marker or validated in the game.
`source-slime-tree-config.rs` reads the original simple-state providers and
their properties, including sky-vine `stage=middle`; missing block states or
properties fail before tree writes. Other provider types are not approximated.
`source-slime-island-markers.rs` models source weighted selection and the
per-piece limit of three successful trees, including skipped draws and failed
placements. `source-slime-island-marker-world.rs` connects tree markers to
the tree writer and shared budget, and implements weighted grass selection and
initial vine attachment/growth. In-memory tests cover three successful trees
and a skipped fourth, valid/invalid grass soil, and a supported growing vine.
`source-slime-island-piece.rs` invokes these handlers from the source template
markers and saves the chosen configuration, rotation, mirror and tree count.
A synthetic-template test covers piece state round-tripping. The saved-start
hook now reconstructs this piece and preserves other structure sets; no start
hook creates it yet, so in-game restoration has not been tested.

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
checking. The saved-start hook has since been connected to the source piece;
the start probe remains a no-op and neither proves in-game save/reload.
An adapter over `dyn PieceWorld` type-checks for basic block and height
access, but the piece context does not expose the host's named-feature
catalog or complete tree state-provider bindings. The retained original
`tconstruct:slime_tree` definition currently returns `Field("basic feature
type")` from the host feature decoder. A vanilla tree is not a substitute
for the source algorithm and is not installed here.
