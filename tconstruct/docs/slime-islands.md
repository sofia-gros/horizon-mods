# Slime islands

The reference definitions and 25 island templates are pinned to TConstruct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. This candidate connects partial
Earth and Sky ocean, Sky and Clay inland, Blood Nether, and End island sets.
Structure placement, saved-piece restoration, slime tree and vegetation
handlers are implementation candidates; natural generation, acquisition and
save/reload have not been established for this integration.

The source-specific slime-fluid blocks are not represented by the targeted
host fluid implementation. Installed templates omit those fluid positions and
preserve the existing world blocks there; they do not substitute water, lava or
another fluid. This is an incomplete island pool, and already-generated chunks
are not automatically repaired when a missing feature is later added.

The terrain-face tint injection is deferred after an unsupported binding;
color maps and their lookup code are retained. Ender island tree markers,
slime mobs and their spawn overrides are not connected. The live planted Ender
propagule path is a separate, unverified candidate. See the sections below
for the exact supported paths and test requirements.

## Earth and sky foliage preparation

Earth and Sky foliage and Sky and Ender vines use the host's
`minecraft:short_grass` block as their sound template. The source assigns grass
sounds to these blocks; Horizon's `sounds` property takes a registered block ID
rather than a sound-event ID. Formal resource preparation and gameplay with
these declarations remain to be verified.

The Earth and Sky leaves, saplings, fern and tall grass, plus Sky vine, now
have content declarations, the pinned source assets and loot tables. Leaf
loot preserves the source sapling and slime-ball Fortune chances; vanilla
shears and Silk Touch are supported, while Forge's extensible `shears_dig`
tool action is not yet represented. Sky vine declares the source three-stage
property in addition to its template's five face properties. These are
preparation for the island marker and tree code, not proof of natural island
generation or acquisition.

Source-specific sapling, leaf, grass and vine updates are experimental and
unverified; position-dependent foliage terrain tint is disconnected. Inland,
Nether and End island sets are partial candidates. Source fluids and
save/reload remain open. The target registry, model and
loot loading, face/stage combinations, tree placement, source drop conditions
and vanilla plant regressions all require dedicated validation.
Earth, Sky and Ender slime leaves are registered and added to
`minecraft:leaves` so the tree writer can recognize them as replaceable.

The four Earth/Sky slime-grass combinations present in the Earth and Sky
island templates are also registered with their pinned source models and
Silk Touch versus underlying slime-dirt loot. Source grass/nylium random ticks
have experimental connections described below; position-dependent terrain tint
is disconnected, and dedicated validation remains open.

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
Plant random-tick and the partial inland, End and Nether island candidates
require validation. Source fluids, slime mobs and some tree markers remain
unconnected. The target-version injections, loading, placement and saved-start
restoration require dedicated plan, build and in-game validation; no natural
generation or survival acquisition is claimed until that succeeds.

## Experimental foliage tint

The five original color maps and lookup code remain available, but the
terrain-face hook is disconnected following the b53 `E_INVALID_BINDING`
diagnostic for the destructured `Mesher::face` argument. Position-dependent
terrain, item and particle tint are not established by this candidate. Color-map
indexing, resource reload and shader behavior still need validation.

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
using vanilla distance rules. Ender leaves are registered; dedicated-server
behavior remains incomplete. Sky-vine support, sapling growth and grass
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
does not use the overworld grass-spreading branch. Grass bonemeal has an experimental hook described below. Nylium bonemeal and
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

## Historical combined validation candidate

This earlier candidate was based on repository main `283cc1a21fd7880a05312400b5d8e531027d442e`.
Its source dependencies are #191 → #200, followed by #204, #206 → #218,
#208 and #212 → #215. It also includes #173's congealed-slime loot.
Its TConstruct target was Horizon `9a6f7dd5a193d709f389adeb1f51858465b8f747`
and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. Its staging instructions
required a unified PlutoLib bundle for that exact host from tree
`aa8a6068ae9f24938f935a665733bf67cac53fea`; that branch did not supply or verify
the bundle or claim a combined plan, stage, build or game result.

## Experimental Ender slime vine

The source Ender slime vine now shares Sky vine's source three-stage growth,
placement and support logic while retaining its own identity: only a vine of
the same kind above supports a hanging side. It uses the original Ender vine
assets, shears loot and Ender foliage color map. Both registered slime vines
use the source hardness and resistance of 0.75. The Ender vine and its
natural acquisition have not been tested; Ender leaves and islands do not yet form a complete generation route. Forge's extended shears
action and dedicated-server behavior remain open.

## Target and validation

The [current target](../compatibility.json) is Horizon 0c7b with Mixins ddd.
The earlier fixed integration in [#354](https://github.com/sofia-gros/horizon-mods/pull/354)
passed an official apply and client build on
Horizon `0c7b5b61a5e746d621afffba919b12aeccf9c0fa` and Mixins
`ddd9700b21997c95d95ac6ee5ecafe3830cf622a` with its matching unified
PlutoLib bundle. That result does not establish this candidate's apply, build
or gameplay. [#361's plan report](https://github.com/sofia-gros/horizon-mods/pull/361#issuecomment-6594377397)
records a plan pass for `aeef32affb1c8f0145a1e4d07e8b6ccdd6670b73`,
not a completed apply, build or game pass. Natural generation, source fluids,
save/reload and LAN require separate validation.

On Horizon `b193a877110ac1505227e6e057b40aade8aac8dd`, the pinned Mixins
transformer reformats the host's valid `(0. ..=1.)` frequency range as invalid
`0...=1.` when processing the island destination file. The targeted host or
Mixins formatter needs a compatible spelling or serialization before an
unmodified-source latest-host build can pass. This is separate from the 0c7b
verification and does not disable the island integration.

## Experimental inland, Nether and End terrain

The original inland Sky/Clay, Nether Blood and End island structure sets,
biome tags, weighted definitions and source templates are connected as
partial terrain. Ender/Blood fern and grass registrations supply their marker
choices. Source Ender slime and magma fluid palette entries are ignored at
placement, leaving pre-existing blocks at those positions. Blood fungus markers
have an experimental source-derived placement path; selected unsupported
vanilla and Ender trees remain deferred. Missing features change later marker
RNG and leave generated chunks incomplete. Updating the mod does not
automatically repair those chunks, so use a disposable world for validation.

This is not complete source generation: source fluids, deferred tree features,
fungus parity, slime mob overrides and several plant behaviors remain open.
The reference templates remain untouched. Target Mixins plan, loading, biome
selection, weighted placement, markers, terrain, block drops, missing-fluid
positions, save/reload and local/LAN regressions require dedicated validation.

## Experimental Ender leaves and propagules

Ender slime leaves and propagules are registered with source assets, source
loot and slimy-leaf tag membership. Leaf distance/decay uses the candidate's
slime-leaf rules. A leaf can grow a hanging propagule with bone meal; hanging
propagules age from 0 to 4 and require slimy-leaf support. Planted propagules
use slime soil, the source one-in-seven random advancement, and the source
bone-meal success chance. This candidate connects stage-1 growth to the Ender
tree placement code; it retains the water at the sapling position and restores
the sapling if placement fails. Growth is deferred when the surrounding
chunks are not loaded, which is a conservative difference from the source.
Exact waterlogging, placement,
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

This section describes planted growth; the separate experimental island Blood
fungus path is described below. Compare the host writer's block-update flags,
source replaceability and RNG against TConstruct before claiming parity. Tag loading, all feature
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

The unsupported terrain-face tint injection remains omitted. Island support
modules use ordinary comments so the Mixins loader does not interpret their
headers as file attributes. The earlier fixed integration passed apply and a
client build on 0c7b/ddd; this candidate requires its own verification.

## Ender tree components

The original Enderbark root block, item, loot and axe tag are registered. Its
`minecraft:mangrove_roots` template inherits the host's same-block vertical
face-culling rule, matching the pinned source rule. A prepared-resource probe
checked both root states, all six directions and different-root controls with
the host `FaceVisibility` implementation. This establishes that the candidate
resource data follows that visibility path, not that a final build's pixels
have been checked; model, mesh and in-game validation remain open. A shared
Ender-tree component provides the four source congealed-slime/root
replacement mappings and the original leaf-vine decorator's ordered 0.125
random draws and maximum four-block hanging extensions. The component is
used by the candidate live-world tree implementation. Its soil provider keeps
source dirt except grass blocks and mycelium, and replaces other blocks with
dirt; the source Forge `onTreeGrow` extension for third-party blocks is not
implemented. Roots use the host water-state query. The island marker remains
deferred: the host's `PieceWorld` interface does not expose the shape-edge
updates or block-tag queries needed by tree placement. On the b53 host,
`structure::legacy::PieceWorld` would need object-safe forwarding methods for
`PlacementContext::update_shape_at_face(position, direction, flags)` and
`PlacementContext::block_matches_tag(position, tag)`, with the existing blanket
`PieceWorld for W` implementation delegating to `W: PlacementContext`. The
underlying world implements these operations, but `PieceContext.world` erases
that interface. The pinned Mixins `WRAP_OPERATION` also requires a target
parameter as a method receiver, while this call uses the local piece `p`.
The live connection, root placement,
waterlogging, leaf ordering and random draws require official apply, type,
package and in-game verification. No Ender tree growth is claimed as tested.

The b53 selector and module-header follow-ups reached `E_INVALID_BINDING`
for the destructured `Mesher::face` arguments. The integration omits that
terrain-tint injection only; vegetation, island generation and source lookup
remain available. Do not infer a tint implementation from map loading.

Earlier Horizon b53 fixtures resolved ten foliage sound declarations to known
grass sounds and checked Earth/Sky island generation plus Earth foliage harvest
and reload ([image](slime-foliage-sound-test.png)). Audio was not listened to;
other foliage and vine placements were not verified. This is not evidence for
the current build.

The position-dependent terrain tint hook remains disconnected. The former
`Mesher::tint` site also destructures its inputs, and `Tints::get` lacks the
block identity needed for a direct substitute. A compatible hook needs separate
investigation and validation.

## Historical implementation evidence

The [pre-reconciliation island notes](https://github.com/sofia-gros/horizon-mods/blob/aeef32affb1c8f0145a1e4d07e8b6ccdd6670b73/tconstruct/docs/slime-islands.md)
retain the earlier implementation sequence and probe details. They describe
older targets, not the current feature status:

- Source Overworld sky placement uses spacing 35, separation 4, frequency 0.5
  and Sky:Clay weights 4:1. Placement tests cover rotation offsets and four
  surface samples; marker tests cover three successful trees, a skipped fourth,
  soil rejection, grass/vine handling and saved-piece round-tripping.
- Generic `place_in_chunk` and closure-contained call probes reported
  `E_UNSUPPORTED_SIGNATURE` and `E_UNSUPPORTED_CONTROL_FLOW`. Limited HEAD
  probes for `try_generate` and `StructureState::use_saved_start` passed
  transformation and `horizon-world` type checks. Those isolated checks did not
  establish natural generation, save/reload or the later integrated build.
- A `FluidKind` state probe reported `E_BRIDGE_TARGET`; a struct state bridge
  does not add fluid enum variants. Earlier content-state restrictions were
  investigated against host `e7f4440f05461c8975bebd2d185e3e9c6214aa4a`;
  those old restrictions are not evidence that today's registered vine states
  are absent. No vanilla fluid or tree substitution establishes source parity.
- The historical b53 follow-up was based on main
  `61613253e4aa06eb2e739a9f6343a93652f014f8`, targeting Horizon
  `b53cfcd14eab7b6ed2fe91a3a3773d7415402ab9` with ddd and its matching
  unified PlutoLib bundle. Other joint b53 work did not verify that candidate;
  earlier #220 and #223 heads were unchanged.
- The b53 apply for `f0b9ee660e22fb35517e798d053d0f3672d6136b`
  reported `E_SELECTOR`; ordinal zero with `require = 1` addressed that selector.
  A later `E_MODULE` led to ordinary rather than inner-doc module comments.
  The subsequent `E_INVALID_BINDING` left only the terrain-face tint injection
  deferred. These intermediate fixes are not independent build/game passes.

## Experimental slime plant support

The authoritative level's survival hook recognizes all five slime fern and
short-grass variants. These plants can remain on any of the four slime dirts
or 25 slime-grass combinations, and reject ordinary dirt and unrelated
blocks. This connects support checks; placement, neighbor updates, drops,
dedicated-server, save/reload, and LAN parity remain to be validated.

## Experimental slime-grass bonemeal

The authoritative hook accepts bone meal on Earth, Sky, and Ender slime grass
when the block above is air, then uses the pinned grass algorithm to attempt
up to 128 fern or tall-grass placements on `tconstruct:slimy_grass`. The
original tag and its three foliage variants are supplied. Attempts that cross
unloaded chunks are skipped conservatively, so chunk-edge results can differ.
This does not implement Blood or Ichor nylium bonemeal, which also spreads to
soil and may grow saplings. Datapack tag reload, obstruction, consumption,
placement and survival, save/reload, local/LAN and dedicated-server behavior
remain unverified in the running game.

## Experimental slime-grass seeds

The five seed items and their pinned textures are registered in the blocks
creative tab. On the authoritative server, a seed converts the clicked dirt
or slime dirt to the corresponding grass or nylium. Sky and Ender seeds also
convert a vanilla vine, preserving its faces. A failed write does not consume
the seed; creative mode keeps it. This is an intentional safety difference
from the original, which consumes after attempting the write.

Natural acquisition from source loot injectors is not implemented. The
server's target permission checks precede the item-use hook, but full host
integration and game tests are still needed for protected targets, adventure
mode, offhand, sounds, save/reload and LAN. Forge tool/action behavior is a
separate limitation.

### Pig Iron piston reaction

The source Pig Iron block is an `OrientableBlock` built with ordinary metal
properties. Its four-facing Horizon template is glazed terracotta, whose
push-only reaction would prevent sticky-piston retraction. Resource preparation
now changes only the four Pig Iron gameplay rows to `PUSH_PULL`; malformed or
missing rows fail preparation instead of changing unrelated blocks. The owned
startup cache generation is bumped so existing owned startup caches are rebuilt.
Previously hand-prepared resource roots must be prepared again to receive the fix.
If manual preparation fails, do not use its incomplete output; rerun preparation
with a new output directory. This has not been tested in a running game or over LAN.
