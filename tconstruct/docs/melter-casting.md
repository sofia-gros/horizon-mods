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

A player break of a Melter, Smeltery or Foundry controller is canceled when an
existing readable machine source contains unknown or non-round-tripping
metadata. This limited pre-drop check avoids clearing such data on that path;
it can also block breaking noncanonical but harmless old state. A localized
action-bar message reports the refusal to the acting player. It runs before
Overwield's break accounting. Missing source records with a nonempty or unknown live inventory are also
rejected; empty or absent live inventory without a saved source remains allowed.
Serialized outer inventory items must round-trip. Only the host's exact empty
`components` compound and `keepPacked: 0b` envelope are exempt from unknown-field
protection. Melter slots must match the live inventory exactly.

Smeltery and Foundry source writes validate the old live projection before
changing either authoritative inputs or their UI view. The shared store stages
both changes together: a closed view is retired to 54 empty slots, while an
open view is updated at its known page offset with its separately owned bucket
slots. Guest and runtime writes use that same transaction. Failed validation
or any failed member of a multi-source batch leaves source and live data
unchanged. Local UI, runtime and chute paths also check before they can
replace a live view. Menu operations restore the validated old page before
committing their new authoritative inputs through the store. Guest Pickup owns
its final commit and restores cursor, inventory and drag on failure, withholding
speculative drops. Retained-wire incoming items are refused before serialization.

The outer `tconstruct:retired_projection` field accepts only integer modes
`0` (open/unproven) and `1` (retired). Mode 1 can authorize only an empty
serialized Items list plus 54 empty live slots. A marker never authorizes
discarding nonempty live contents absent from current saved inputs. Historical
snapshot compounds, unknown versions and extra metadata are refused, even if
a forged snapshot happens to match the current live items.

On reload, a nonempty live view must exactly match both serialized Items and
a complete legal page of current authoritative inputs, with empty container
slots. No guessed default offset, bag/subset comparison or historical-cache
trust is used. The next source write retires a closed loaded view atomically
before changing its authoritative inputs. Mismatched legacy saves remain
protected, including old saves whose cached page was already stale or whose
high slots had already been truncated. The decoder now retains all 54
Smeltery slots; already absent data cannot be reconstructed.

Pending bucket slots remain protected after reload even if their in-memory
owner registry is gone. Opening refuses unexplained or opaque live data
rather than overwriting it. This is not a recovery procedure for old damaged
or ambiguous saves. The guard covers only player destruction, not explosions,
pistons, commands or other replacement. Actual in-world item drops and LAN
behavior remain unverified.

Run the source-based Rust fixtures with:
`HORIZON_HOST=/path/to/clean/pinned/horizon bash tconstruct/tests/run_machine_destroy_guard_rust.sh`.
The runner requires the pinned host, Rust/Cargo and cached dependencies, checks
dependency identities against the host lockfile, and verifies selected test
names so a zero-test run cannot pass. The suite exercises real item/NBT codecs,
host overlay/canonicalization, the decode adapter and store/CAS code in a
minimal world fixture. These tests pass; they are not a real world/LAN test or
a substitute for type-checking the full injected host.

The guest Pickup transaction helper is also executed with a simulated native
menu engine and the real store, forcing final-write failures for deposit,
withdrawal and outside drop. These tests establish rollback in the adapter,
not end-to-end native LAN behavior. The separate store fixture can be run with
`MACHINE_GUARD_FIXTURE=smeltery_nbt_store.rs` using the same runner.
