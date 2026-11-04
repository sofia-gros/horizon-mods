# Melting and casting

Use a Melter, a Seared Heater, a Faucet and a Casting Table or Basin to melt and cast supported materials. Place the heater directly below the Melter, and the faucet beside the Melter above the table or basin.

Open the Melter, put a supported fuel in the heater slot, and add material to the three input slots. For example, ten copper ingots produce 900 mB of molten copper.

Solid fuel consumption and Melter state are staged together. A fuel remainder returns to the heater only if it is itself valid heater fuel and fits; otherwise it drops at the heater, as in the source. If the Melter becomes unformed while fuel remains, that fuel burns down without heating or cooling its inputs; cooling resumes after it runs out. These additions need game, persistence and LAN verification.

![Melter containing molten copper](melter-filled.png)

The input slots are on the left, the heater slot is on the right, and the tank shows the stored liquid.

Place an ingot cast on the table, use the faucet, and wait for the copper ingot to cool. Use an empty basin to cast 810 mB into a copper block (nine ingots). Use the table or basin to collect the finished item.

The copper sequence, saving and reloading, and shared input and casting in LAN have been checked on Linux. Other materials and Windows have not received the same game checks.

Forge milk is supported as a machine-tank fluid for skull-melting lookup and milk bucket transfers. Milk-tag ingot casting now has a registered Cheese Ingot result; the source recipes are retained. Cheese Block is registered with 2×2 ingot conversions. Its source basin recipe requires the `forge:milk` tag; availability through the packaged source layers is under separate verification. The block item is not edible yet, to prevent losing the three-ingot remainder. Cheese Ingot restores hunger and saturation, but its source effect-removal behavior is not yet connected. Cheese Block uses source walk and jump factors, but its lighting and rendered inner planes are not yet verified in game. The original Forge milk sprite is staged for still and flowing milk. These paths have focused tests but have not been checked in game, persistence or LAN.

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
