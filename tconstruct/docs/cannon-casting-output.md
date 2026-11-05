# Fluid Cannon supply into Casting Tank containers

Issue #379, stacked on #807 / #766 / #728. A Seared Fluid Cannon can now supply
a Casting Tank through a Faucet, including staged destination bucket/bottle
processing. This adds fluid extraction, not Cannon firing or item automation.

The Cannon input adapter is constructed only in this route. It requires canonical
saved state, a matching live item and no retained wire backing. Only its nested
tank is patched, leaving the saved item and live inventory untouched. Destination
item backing is also guarded. Source and destination snapshots, blocks, entities,
chunks and live items are rechecked after destination container processing.
Source/Faucet/destination changes commit together before live destination items
and comparator notifications are updated. Failed saves do not publish them.

The existing transfer policy preserves exact fluid/tag identity and reserved
packets, pours immediately on first and later reservations, and respects the
4,000 mB destination capacity. Registered vanilla base potion tags work in
network contexts; supported custom potion data remains local-only. Source tank
extraction follows the fixed upstream all-face fluid capability:
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/FluidCannonBlockEntity.java
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/component/TankBlockEntity.java

Four fixture cases and wiring coverage were added for item preservation and
bucket completion, packet timing, eight save/source/callback refusal states,
base potion bottles and local custom color versus network refusal, and the
final 5 mB of capacity. Tests were not run. Review was a brief static pass and
diff whitespace check only. No CI, version checks or merge were performed.

Other Cannon output destinations, generic capabilities, firing, redstone
scheduling, actual host execution, datapack reload, save/reload and LAN remain
separate or unverified. World/storage and datapack context are mocked in the
fixture; its container processing uses the production state implementation.
