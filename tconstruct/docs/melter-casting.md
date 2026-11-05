# Melting and casting

Use a Melter, a Seared Heater, a Faucet and a Casting Table or Basin to melt and cast supported materials. Place the heater directly below the Melter, and the faucet beside the Melter above the table or basin.

Open the Melter, put a supported fuel in the heater slot, and add material to the three input slots. For example, ten copper ingots produce 900 mB of molten copper.

Solid fuel consumption and Melter state are staged together. A fuel remainder returns to the heater only if it is itself valid heater fuel and fits; otherwise it drops at the heater, as in the source. If the Melter becomes unformed while fuel remains, that fuel burns down without heating or cooling its inputs; cooling resumes after it runs out. These additions need game, persistence and LAN verification.

![Melter containing molten copper](melter-filled.png)

The input slots are on the left, the heater slot is on the right, and the tank shows the stored liquid.

Place an ingot cast on the table, use the faucet, and wait for the copper ingot to cool. Use an empty basin to cast 810 mB into a copper block (nine ingots). Use the table or basin to collect the finished item.

The copper sequence, saving and reloading, and shared input and casting in LAN have been checked on Linux. Other materials and Windows have not received the same game checks.

Forge milk is supported as a machine-tank fluid for skull-melting lookup and milk bucket transfers. Milk-tag ingot casting now has a registered Cheese Ingot result; the source recipes are retained. Cheese Block is registered with 2×2 ingot conversions. Its source basin recipe requires the `forge:milk` tag; availability through the packaged source layers is under separate verification. With the native runtime active, players can eat the block in air, returning three ingots to the emptied hand or inventory. If a stacked block is eaten with insufficient inventory space, consumption is refused rather than dropping an item that another mod might cancel. Content alone leaves it inedible, preventing loss when the native hook is absent. Both foods restore hunger and saturation, but their source effect-removal behavior is not yet connected. Eating particles and sounds are not yet implemented; item stacks with custom components or damage and nonplayer eating are unsupported. Consumption and remainder are committed before advancement callbacks to avoid losing another item if a callback changes the hand. Full-inventory behavior and LAN remain unverified. Cheese Block uses source walk and jump factors, but its lighting and rendered inner planes are not yet verified in game. The original Forge milk sprite is staged for still and flowing milk. These paths have focused tests but have not been checked in game, persistence or LAN.

The implementation uses the original recipes, the original Melter ore-rate defaults and the source fluid textures where available. Fluids without a retained source texture use a generic visible texture; source animation metadata is now staged for fluid aliases, while the casting cooling fade remains unimplemented. Atlas animation and reload still need in-game verification.

Fluid-container interactions, redstone control and specialized recipe types remain separate work. For a tested small alloy setup, see [Smeltery and alloys](smeltery.md). The [Foundry](foundry.md) can also recover a byproduct from copper ore. The Faucet, Casting Basin and Casting Table use the original selection and collision shapes. A Linux game check covered all five Faucet directions, right-click targeting, the Basin and Table recesses, walking against one Faucet, and falling into the Basin and Table. The Melter screen still opened. Other machine shapes remain to be compared.

![Casting Basin selection in the Linux test world](casting-basin-selection.png)

For acquisition from gathered resources and a fixture-free acceptance route, see [Survival progression](survival-progression.md).

## Filling a lantern

Seared and Scorched Lanterns hold 50 mB. Their source filling recipes transfer
10 mB on a Casting Table and up to 50 mB on a Casting Basin, limited by the
space remaining in the lantern. A different fluid must not replace fluid
already stored inside it.

A representative Linux Table test transferred 10 mB, left 990 mB in the source
tank, and retained the lantern's contents after restart. The Basin test did not
transfer fluid. One identified cause is an unresolved empty-cast tag referring
to an unregistered gold platform. The corrected entry checks the original tag
graph and accepts only an input proven not to belong to that empty-cast tag;
unknown membership and cycles still refuse the operation. Focused recipe and
tag tests pass, but a game rerun of the corrected Basin path is still pending.

![Lantern filled on the Casting Table](test-lantern-table.png)

The fluid animation manifest records the original `.mcmeta` entries from TConstruct
revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, including frame timing and
interpolation. Powdered snow has no source animation metadata. Both the manual
resource overlay and the content package stage metadata beside each alias.
Tests check packaging and values, not visible animation or resource reload.

### Smeltery machine metadata

Writes through the shared metadata store preserve unknown top-level fields in
the existing `tconstruct:smeltery` machine compound while replacing the
owned fields according to the current operation, including clearing omitted
optional `heat_codes`. For Smeltery and Foundry, unknown keys directly on the
native tank compound are also retained. Legacy `FluidName`, `Amount` and `Tag`
aliases omitted by a serialized update are not retained. Raw patch updates that
explicitly include them keep them, as before, and may leave stale aliases. Fluid entry fields, slot payloads, other legacy Forge formats, and
collisions with owned field names remain outside this protection. It cannot
restore data already lost. A non-compound existing machine value rejects an update; malformed owned
fields may still be normalized by their existing decoder. Client network
snapshots retain replacement semantics rather than merging old fields. Cached-host-type
fixture tests exercise the merge and the shared store; full host compilation,
real world save/reload, all interactions, and LAN remain unverified.

A tank containing a fluid ID with an unpaired UTF-16 surrogate cannot be
rewritten through the lossy native decoder. Store updates are rejected unless
the raw tank is unchanged. This preserves that identifier during those writes
but can pause all machine ticks. Client LAN snapshot replacement bypasses this
guard. Controller destruction is not protected and neither tank data nor
inventory drops are guaranteed. Other malformed or unknown nested data is
outside this check. Host integration and world/LAN behavior remain unverified.

On the Melter tick's preserving save, and other callers of the preserving
single-tank encoder, an unpaired UTF-16 surrogate in the old fluid ID rejects
the update rather than being rewritten lossily. This may pause those machines.
Other non-preserving transfer, destroy and client snapshot paths are not covered.
The cached-type fixture is not a full host, world, save/reload or LAN test.

For Smeltery/Foundry fuel draw and direct player fuel-tank interaction, the
native tank compound is patched on a staged copy: unrelated top-level fields
are retained as parsed NBT values, not necessarily identical bytes. They remain
when the tank empties or another fluid is inserted, so their meaning may become
inconsistent if they depended on the old fluid. Legacy `FluidName`, `Amount` and `Tag` aliases are dropped
as in the former serializer. An invalid UTF-16 fluid ID blocks that candidate
without consuming fuel or the held container. Other transfer, drop and old
Forge formats remain outside this limited change. Host/world/save/LAN behavior
is unverified.

Faucet transfers between supported tanks refuse to patch a source, target or
reserved packet whose fluid ID cannot be converted from UTF-16 without loss.
Their staged writes leave fluid and packet data unchanged on this refusal. If
an invalid source remains present, even an already reserved packet can wait
without pouring until the source is corrected or removed. Older casting table,
basin and cannon routes and broken/replaced blocks are outside this guard.
Fixture tests use mock world and atomic store behavior; host integration and
LAN remain unverified.

### Held casting containers: native extension preservation (#379)

Held-container filling and draining on Seared/Scorched Casting Tables and
Basins now use the preserving casting codec instead of requiring an exact
canonical serialization. Native machine/tank extension fields and unchanged
opaque saved item fields survive supported transfers. Source/Forge aliases,
malformed owned fields, lossy strings and wire-backed live items still refuse
the exchange. Lantern items retain their existing portable-data safety gate.

The exchange is staged before checking the original saved state, live casting
slots, block state, loaded chunk, held stack and creative mode again. The
casting save must succeed before replacing the held container or giving its
remainder. Cooling and ordinary item-slot behavior remain unchanged.

The bounded interaction fixture adds all four blocks, filling/draining,
single/stacked containers, creative behavior, nine changed-state cases and
invalid metadata/save cases. World, recipe lookup and portable conversion are
mocked; the interaction, casting codec, item codec and metadata store are the
production implementations. These cases have not been run in this batch.
Full host/world, real portable conversion, save/reload and LAN validation are
still deferred to the combined pass.

### Portable Lantern native extensions (#379)

The shared Seared/Scorched Lantern item-transfer path now patches its three
native tank fields rather than discarding the tank compound. Filling, partial
draining and full draining preserve tank extensions, surrounding custom data
and other item components. An empty tank containing extensions remains saved
and can be refilled; an ordinary empty tank still removes its payload. The
50 mB per-item capacity and one-item-per-transfer stack handling are unchanged.

Casting and Melter held-container gates accept these preserving Lanterns.
Malformed native data, lossy fluid IDs, legacy tank aliases and both current
and stale retained wire bytes are rejected before transfer. Other portable
tank types retain their previous no-extension decoding restriction. Native
state tests were added for both Lanterns, potion identity, fill/drain/refill,
stack splitting, alias/wire refusal and the non-mutating Melter plan. They
were not run in this batch. Host/world, save/reload and LAN validation remain
for the combined pass; placed Lantern transfers are separate PRs.

### Copper Can identity and retained-wire guard (#379)

Copper Can's shared decoder now requires an exact UTF-16 fluid identifier and
refuses current or stale retained wire backing. This prevents permissive
registry callbacks or direct setters from replacing a lossy ID or rewriting
an opaque item. Transfers, Melter plans, displays, container acceptance and
crafting-remainder predicates all use this boundary.

Valid cans retain their 90 mB quantity, exact fluid tags, native extension
data, other item components and one-item stack splitting. Three production
unit cases cover lossy IDs across entrypoints, both wire states for filling
and draining, and valid named/extended stacks with distinct potion layers.
They were not run. Real inventory/menu interaction, save/reload and LAN remain
for the final combined pass. No new fluid types or external capabilities are
introduced by this change.

### Casting item fallback after a fluid probe

Datapack preparation is followed by the same exact snapshot guard whether a
held-fluid conversion succeeds or fails. A changed block/chunk, raw casting
state, live slot, held count/damage/components/wire data, Creative state or
local/LAN mode consumes the interaction without applying a stale item fallback.
The guard runs before conversion, before a successful fluid save, and before
falling through to insertion, cast/result pickup or molding. Unchanged ordinary
item behavior and preserving writes remain intact.

The extracted production interaction fixture adds all four Table/Basin variants
and five fallback modes, with thirteen preparation changes and unchanged controls.
State/store/item codecs are real; recipes, portable conversion, acting hand and
world callbacks are fixtures. Cases are added but unrun. Full host/world/LAN and
post-save crafted-item callback behavior remain outside this change.
