# Honey Bottle fluid transfer

A normal Honey Bottle can now supply 250 mB of stock Honey fluid to the existing
container-transfer paths, returning one Glass Bottle. The shared planner is used
by heating-machine container slots and single-fluid tanks. The direct Melter
interaction added in #655 uses the same planner and saves its tank before applying
the inventory result. This makes bottled honey a fluid supply without first
crafting and melting a Honey Block; other Pig Iron ingredients and alloying
requirements are unchanged.

One input bottle is processed per transfer. Stacked inputs keep the remaining
bottles, and container input slots keep the vanilla 16-item Honey Bottle limit.
A full tank, occupied output slot, missing item/fluid registration, incompatible
single-tank fluid, fill-only mode, damaged item or unrepresentable item metadata
leaves the input and tank unchanged. The Melter planner preserves non-tank machine
data and retained tank extension fields. Existing Honey-to-bottle filling remains
250 mB, so the stock round trip does not create fluid or items.

## Source and scope

The source is MIT-licensed Mantle at
[`17c287ad`](https://github.com/SlimeKnights/Mantle/blob/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/generated/resources/data/mantle/mantle/fluid_transfer/honey_bottle_empty.json):
Honey Bottle → 250 mB from `forge:honey` + Glass Bottle. The fixture retains that
source recipe. Attribution is in
[`honey-bottle-transfer-NOTICE.md`](../third_party/honey-bottle-transfer-NOTICE.md).
This bounded implementation selects bundled `tconstruct:honey`,
matching the stock tag pair and existing filling path. Arbitrary modpack tag
replacement or another mod's Honey fluid preference is not implemented.

Rust regressions cover amount, bottle return, round trip, stack handling,
no-mutation failures, incompatible single-fluid tanks and preserved Melter NBT.
The Python fixture check covers the pinned transfer and bundled fluid tags.
Actual in-game interactions, rendering, save/reload and LAN remain unverified.
Direct Melter transfers retain #655's local-host restrictions; this change does
not expand its multiplayer support.
