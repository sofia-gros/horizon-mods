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

Source-specific sapling, leaf, grass and vine updates and foliage terrain tint
are experimental and unverified. Other island sets, source fluids and
save/reload remain open. The target registry, model and
loot loading, face/stage combinations, tree placement, source drop conditions
and vanilla plant regressions all require dedicated validation.
The two registered leaves are added to `minecraft:leaves` so the existing
tree writer can recognize them as replaceable; other original slime leaves
are not registered or added by this batch.

The four Earth/Sky slime-grass combinations present in the Earth and Sky
island templates are also registered with their pinned source models and
Silk Touch versus underlying slime-dirt loot. Source grass/nylium random ticks
and position-dependent terrain tint have experimental connections described
below; bonemeal and validation remain open.

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

Custom slime mobs and their spawn overrides and item tint are still missing.
Plant random-tick candidates require validation. Sky-inland/clay, End and Nether sets remain
unconnected because their original feature types and resources are not yet
implemented. The target-version injections, loading, placement and saved-start
restoration require dedicated plan, build and in-game validation; no natural
generation or survival acquisition is claimed until that succeeds.

## Experimental foliage tint

The five original color maps are loaded from the active render resource root.
For the registered slime grasses, fern, tall grass, leaves and Sky vine, a
terrain-face tint hook uses the original 256-block coordinate reflection,
including the original 128-block offset for leaves and vines. The maps are
reloaded when render assets load. Missing or invalid maps leave the host tint
unchanged. Item tint, particle tint and resource-reload behavior still require
implementation or validation. The target Mixins plan, shader compatibility,
color-map indexing and actual world rendering have not been tested.

## Experimental slime leaf lifecycle

Earth and Sky slime leaves now request random ticks. A source-specific
neighbor-update hook schedules their distance updates, and a scheduled-tick
hook computes distance only from `tconstruct:slimy_logs` and the original
Earth, Sky and Ender slime-leaf IDs. The registered source log tag includes
all four original slimewood log/wood families. When distance is seven and the
leaf is not persistent, the existing leaf random-tick path handles decay and
block loot. Normal vanilla leaves keep their existing behavior.

If the slime-log tag or a required state cannot be resolved, the source hook
keeps the leaf state rather than falling through to vanilla's different log
rules. Placement requests persistent leaves using the same source distance calculation;
if required source data is unavailable, placement is refused rather than
using vanilla distance rules. Ender leaf registration and dedicated-server
behavior remain incomplete; Sky-vine support, sapling growth and grass
spreading are experimental candidates requiring validation. The target injection plan, source tag resolution,
placed/persistent leaves, log removal, loot, cross-chunk behavior and
single-player/LAN/save regression require dedicated validation.

## Experimental planted slime trees

Earth and Sky saplings now have source soil survival, two-stage random
advancement and bone-meal hooks. Their runtime tree writer reuses the same
source height and canopy calculations as the island writer, including the
source Earth 4+0..2 height and Sky 5+0..3 height with its one-in-ten doubling;
planted Sky trees do not have the island tree's vines. It checks the source
slimy-soil and leaves tags, only replaces air or leaves, and declines a tree
at an unloaded chunk boundary rather than writing to ungenerated chunks. That
loaded-area guard is a conservative runtime limitation.

The target Mixins plan, type compatibility, normal and bone-meal growth, RNG
order, blocked placement, cross-chunk behavior, soil survival, original leaf
distance, save/reload and local/LAN behavior require dedicated verification.
Dedicated-server growth, other sapling types and tree feature integration for
other island sets remain open.

## Experimental slime grass random ticks

The registered slime grasses request random ticks. The local authoritative
server hook applies the source grass validity, light and four spread attempts
to vanilla and slime dirt, including the source loaded-area and water checks.
The source Nylium variant reverts to its underlying dirt when blocked; it
does not use the overworld grass-spreading branch. Source grass bonemeal and
dedicated-server coverage remain open; sapling and vine growth are separate
experimental candidates. The target injection plan, exact light/occlusion and random-draw order,
chunk edges, snow, fluids, save/reload and vanilla grass regressions require
dedicated validation.

## Experimental Sky slime-vine updates

The Sky vine requests random ticks and uses the source side-attachment,
three-stage downward growth, floating-length and random-draw rules. A neighbor
update preserves sides supported by the source slime-leaf tag or by a
non-terminal vine above, and removes unsupported vines. The source tag includes
Earth and Sky leaves; its Ender entry is registered by the propagule follow-up. Ender vines, full island generation, the target
injection plan, support shapes, world and save behavior, RNG order and
single-player/LAN validation remain open. Vanilla vine behavior is unchanged.

Sky vine placement additionally follows the source ordered look directions,
existing-vine side checks and source attachment rules. It uses a narrow bridge
to the host placement context's direction ordering. The bridge plan, placing
on each face, replacement, no-support rejection, player/dispenser behavior,
item consumption and local/LAN regression require dedicated validation.

## Combined validation candidate

The combined branch is based on repository main `283cc1a21fd7880a05312400b5d8e531027d442e`.
Its source dependencies are #191 → #200, followed by #204, #206 → #218,
#208 and #212 → #215. It also includes #173's congealed-slime loot.
The TConstruct target is Horizon `9a6f7dd5a193d709f389adeb1f51858465b8f747`
and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. Stage it with
the unified PlutoLib bundle built for that exact Horizon target from this
checkout's `pluto-lib` tree `aa8a6068ae9f24938f935a665733bf67cac53fea`;
the bundle itself is not supplied or verified by this integration branch.
No combined plan, stage, build or game result is claimed.

## Experimental Ender slime vine

The source Ender slime vine now shares Sky vine's source three-stage growth,
placement and support logic while retaining its own identity: only a vine of
the same kind above supports a hanging side. It uses the original Ender vine
assets, shears loot and Ender foliage color map. Both registered slime vines
use the source hardness and resistance of 0.75. The Ender vine and its
natural acquisition have not been tested; Ender leaves and islands do not yet form a complete generation route. Forge's extended shears
action and dedicated-server behavior remain open.

## b53 follow-up candidate

The follow-up candidate is based on main `61613253e4aa06eb2e739a9f6343a93652f014f8`
and targets Horizon `b53cfcd14eab7b6ed2fe91a3a3773d7415402ab9` with Mixins
`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. Use the unified PlutoLib
bundle built for that exact host by the joint build owner. The earlier #220
and #223 heads remain unchanged. The joint b53 build for other work is not
verification of this island candidate; its combined official plan, stage,
build, game and survival behavior still need validation. Source slime fluids
remain omitted as described above.

## Experimental inland, Nether and End terrain

The original inland Sky/Clay, Nether Blood and End island structure sets,
biome tags, weighted definitions and source templates are connected as
partial terrain. Ender/Blood fern and grass registrations supply their marker
choices. Source Ender slime and magma fluid palette entries are ignored at
placement, leaving pre-existing blocks at those positions. The marker's chosen
unsupported vanilla tree, Ender tree or Blood fungus is saved with the piece
and its initial marker random draw is consumed; the feature itself is not yet
placed. The missing feature changes later marker RNG and leaves generated
chunks without those trees or fungi. Updating the mod does not automatically
repair chunks already generated, so use a disposable world for validation.

This is not complete source generation: source fluids, the deferred tree and
fungus features, slime mob overrides and several plant behaviors remain open.
The reference templates remain untouched. Target Mixins plan, loading, biome
selection, weighted placement, markers, terrain, block drops, missing-fluid
positions, save/reload and local/LAN regressions require dedicated validation.

## Experimental Ender leaves and propagules

Ender slime leaves and propagules are registered with source assets, source
loot and slimy-leaf tag membership. Leaf distance/decay uses the candidate's
slime-leaf rules. A leaf can grow a hanging propagule with bone meal; hanging
propagules age from 0 to 4 and require slimy-leaf support. Planted propagules
use slime soil, the source one-in-seven random advancement, and the source
bone-meal success chance. Their stage-1 Ender tree feature is still deferred;
no vanilla mangrove is substituted, and successful bone meal at that stage
is consumed without creating the missing tree. Exact waterlogging, placement,
leaf drop predicates, hook ordering, game behavior, persistence and LAN require
validation. Dedicated-server coverage and normal Ender tree acquisition are
not established.

## Experimental planted slime fungi

Blood and Ichor fungus saplings and their source hat blocks are registered.
Bone meal checks the original slimy-soil tag and source 40% success chance,
then calls Horizon's existing huge-fungus placement for a source-derived
configuration matching the underlying block. Blood preserves the source
`planted=true` flag and Ichor preserves `planted=false`. Each of the 33
currently registered original soil blocks has a configured variant, because
the host feature accepts one base block while the original uses a tag. A
third-party addition to that tag has no corresponding variant yet. The
runtime conservatively refuses placement across an unloaded chunk boundary.

This connects planted growth only; the island's Blood fungus marker remains
deferred. Compare the host writer's block-update flags, source replaceability
and RNG against TConstruct before claiming parity. Tag loading, all feature
variants, bone-meal consumption, obstruction, chunk boundaries, source loot,
save/reload, vanilla-fungus regression and local/LAN need dedicated testing.
Dedicated-server support and normal acquisition are not established.

## Experimental Blood island fungus markers

The Blood island's selected fungus marker now attempts the original unplanted
fungus algorithm rather than always skipping it. It uses the source slimy
nylium and grow-through tags, source height, large-fungus and cap RNG order,
and the host's per-state replaceability facts. Successful placements count
against the original three-tree budget. The generation depth is persisted with
the piece; older saved pieces use the current generator depth. If host tree
facts are unavailable, the marker remains deferred. The original Ender and
vanilla tree markers remain deferred, and source fluids are still omitted.

Only syntax checks have been performed. Official target plan/type checking,
structure start and saved-start bindings, tag and state-fact resolution,
block-write boundaries, RNG/placement comparison with the original, tree
budget, save/reload, local/LAN and normal-world tests remain necessary. Use a
disposable world until these checks pass; earlier chunks without fungi are not
retrogened.

The official b53 apply for candidate `f0b9ee660e22fb35517e798d053d0f3672d6136b`
reported `E_SELECTOR` for the foliage face hook because its site ordinal was
missing. The follow-up specifies ordinal zero and retains `require = 1` for
Mesher::face's single `tint` initialization. This source-level correction has
not yet passed the official plan/apply, type check or game validation.

The b53 follow-up apply after the tint selector fix reached support-module
loading but reported `E_MODULE`: four module source files used inner-doc
comments, which are parsed as file attributes. The follow-up changes those
comments to ordinary comments. Its official apply, type check, build and game
behavior remain unverified.

The later Ender-propagule and planted/island-fungus support modules also use
ordinary comments so that the Mixins module loader does not interpret their
headers as file attributes. Their combined official apply remains unverified.

## Ender tree components

The original Enderbark root block, item, loot and axe tag are registered. Its
source-specific vertical adjacent-root face culling is not implemented. A
shared Ender-tree component provides the four source congealed-slime/root
replacement mappings and the original leaf-vine decorator's ordered 0.125
random draws and maximum four-block hanging extensions. The component is
compiled into both worldgen and live-world target modules, but it is **not
connected to either Ender tree placement path yet**. Root placement,
waterlogging, leaf ordering and chunk-write adapters still require integration;
there is no claim of Ender tree growth. The code and block are not yet plan,
type, package or in-game tested.
