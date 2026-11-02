# Slime islands (in progress)

The source definitions and island templates in `content/reference` are pinned
to TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. The original
Overworld sky-island set has spacing 35, separation 4, frequency 0.5 and
weights 4:1 for sky slime and clay islands. Sea and Nether islands and End sky
islands use their own original structure sets and biome tags.

`source-slime-island-placement.rs` implements the original sea height and sky
height calculation, including the rotation-dependent offsets and the four
surface samples. It is tested as a standalone rule and is not connected to
world generation. The 25 original clay, earth, sky, ender and blood island NBT
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
hook now reconstructs this piece and preserves other structure sets. A start
hook selects source templates and supported slime trees. The original overworld
ocean island set is connected as an experimental partial route; in-game
generation and restoration remain untested.

The original uses `tconstruct:island`, not a vanilla jigsaw: its templates
include data markers that can place source slime trees, vines and grass. The
current Horizon structure registry recognizes the type as unsupported.
Tree/foliage registration, original drops and survival acquisition, marker
processing, worldgen integration, save/LAN and real-world validation remain
open. Sky/End slime mob spawn overrides also require their own entity support.
Natural generation and survival acquisition are not yet verified.

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
checking. Both hooks have since been connected to the source piece; their
limited type checks do not prove natural generation or in-game save/reload.
An adapter over `dyn PieceWorld` type-checks for basic block and height
access, but the piece context does not expose the host's named-feature
catalog or complete tree state-provider bindings. The retained original
`tconstruct:slime_tree` definition currently returns `Field("basic feature
type")` from the host feature decoder. A vanilla tree is not a substitute
for the source algorithm and is not installed here.

The source templates also contain custom slime fluid blocks. The pinned host's
`FluidKind` is a closed water/lava enum, and `FluidState::from_block` recognizes
only those vanilla fluid block names. The content block declaration copies a
vanilla template's existing states; it cannot declare the source vine's five
direction properties plus its three-valued `stage`. No vanilla template in the
prepared registry has that property set. A Mixins state probe targeting
`horizon_world::fluid::FluidKind` reports `E_BRIDGE_TARGET` (state targets must
be structs); it does not provide an additional fluid enum variant. Assigning
water to slime fluid or dropping the vine stage would change source behavior,
so neither substitution enables island generation here. These boundaries also
need an implementation for the original clay island's vanilla tree features
and the blood island's fungus; the current start hook rejects unsupported tree
types instead of generating incomplete islands.

The five clay templates use vanilla water rather than custom slime fluid, and
their vanilla grass marker accepts dirt. Clay shares the overworld sky set with
Sky Slime Island at the source 1:4 weight, so enabling clay alone would alter
the source set and is not done. Clay's selected vanilla tree features and its
Terracube spawn override are still unconnected.

The first official host revision with content `state_properties` and
`default_state` is `e7f4440f05461c8975bebd2d185e3e9c6214aa4a`. Relative to
the pinned host it changes 1,533 files; 11 of this mod's 14 module target files
change, and a name-only scan finds 27 injection/constructor targets requiring
inspection. This is a static estimate, not a count of confirmed broken
signatures. The island start's `try_generate` signature and valid-biome API do
change. The host is not upgraded on this draft; custom world-fluid identity,
feature execution, registration, acquisition and validation remain open.

## Earth and sky foliage preparation

The Earth and Sky leaves, saplings, fern and tall grass, plus Sky vine, now
have content declarations, the pinned source assets and loot tables. Leaf
loot preserves the source sapling and slime-ball Fortune chances; vanilla
shears and Silk Touch are supported, while Forge's extensible `shears_dig`
tool action is not yet represented. Sky vine declares the source three-stage
property in addition to its template's five face properties. These are
preparation for the island marker and tree code, not proof of natural island
generation or acquisition.

Random ticks are explicitly disabled on these registrations: source slime
sapling growth, leaf decay and vine growth still need their own behavior, and
inheriting vanilla growth would produce the wrong result. Source foliage tint,
complete template block and fluid registration, other island sets and save/reload remain open. The target registry, model and
loot loading, face/stage combinations, tree placement, source drop conditions
and vanilla plant regressions all require dedicated validation.
The two registered leaves are added to `minecraft:leaves` so the existing
tree writer can recognize them as replaceable; other original slime leaves
are not registered or added by this batch.

The four Earth/Sky slime-grass combinations present in the Earth and Sky
island templates are also registered with their pinned source models and
Silk Touch versus underlying slime-dirt loot. Their random ticks are disabled
until source spreading and bonemeal behavior is connected. Source-tinted rendering remains open; registration alone does not establish island generation.

The original five `slimy_soil` child tags and their aggregate are included,
with all 25 foliage/soil grass combinations, four slimy Enderbark roots and
the existing four slime dirt blocks registered. The source shovel and harvest
tier tags are extended for these blocks. The ground blocks retain source
Silk Touch/underlying-dirt drops, while roots drop themselves. This is
registration and tag wiring; live tag resolution, root behavior, rendering,
mining and normal island acquisition still need dedicated validation.

## Experimental ocean island route

The original overworld ocean set, its two equally weighted island definitions
and its original biome tag are included. Both configured slime trees remain
outside the host's vanilla feature catalog and are read by the island start
from their original JSON. The retained source templates remain unchanged in
the reference archive. The ten installed Earth and Sky template copies replace
only the source Earth/Sky slime-fluid palette entries with structure void; the
piece ignores structure void, leaving the prior world block intact at each of
those positions. No water, lava or inert block is presented as slime fluid.
Consequently pools are absent and existing water or other terrain may remain;
this is partial generation, not source-equivalent fluid behavior.

Custom slime mobs and their spawn overrides, exact block tint and the plant
random ticks are still missing. Sky-inland/clay, End and Nether sets remain
unconnected because their original feature types and resources are not yet
implemented. The target-version injections, loading, placement and saved-start
restoration require dedicated plan, build and in-game validation; no natural
generation or survival acquisition is claimed until that succeeds.

## Experimental Sky slime-vine updates

The Sky vine requests random ticks and uses the source side-attachment,
three-stage downward growth, floating-length and random-draw rules. A neighbor
update preserves sides supported by the source slime-leaf tag or by a
non-terminal vine above, and removes unsupported vines. The source tag includes
Earth and Sky leaves; its Ender entry is optional until Ender leaves are
registered. Vine placement, Ender vines, full island generation, the target
injection plan, support shapes, world and save behavior, RNG order and
single-player/LAN validation remain open. Vanilla vine behavior is unchanged.

Sky vine placement additionally follows the source ordered look directions,
existing-vine side checks and source attachment rules. It uses a narrow bridge
to the host placement context's direction ordering. The bridge plan, placing
on each face, replacement, no-support rejection, player/dispenser behavior,
item consumption and local/LAN regression require dedicated validation.
