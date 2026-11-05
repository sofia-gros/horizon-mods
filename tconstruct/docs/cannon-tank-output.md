# Fluid Cannon output to built-in tanks

Issue #379, stacked on #795 and its Melter/Lantern input stack. A Seared Fluid
Cannon can now supply eleven destinations through a Faucet: the Seared Melter,
both placed Lanterns, and all eight built-in Seared/Scorched fuel and ingot
tanks/gauges. This does not classify Cannons as fuel tanks or change unrelated
source routes.

The fixed upstream FluidCannonBlockEntity inherits the all-face fluid handler
from TankBlockEntity. This is fluid extraction, not Cannon firing:
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/FluidCannonBlockEntity.java
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/component/TankBlockEntity.java

A canonical saved Cannon and matching live item are required. Retained
wire-backed items are refused. Only the nested source tank is patched; its
saved item and live inventory stay unchanged. Source/Faucet/destination state
is saved together after block, entity, chunk, raw and live-item rechecks. Source
comparator and changed notifications follow successful saves. Melter destination
inventory, fuel, heat, progress and extension fields are preserved.

Transfers preserve exact fluid/tag identity, reserve only available destination
capacity (50, 2,160, 4,000 or 4,320 mB), and pour immediately after reservations.
An existing packet is independent of later source refills. Untagged fluids and
registered vanilla base potions are supported; custom potion data waits even
locally. Opaque or missing state and failed saves do not publish staged changes.

Four fixture cases and wiring coverage were added for eleven destinations,
item/metadata preservation, packet timing, committed source comparator changes,
seven refusal states, all capacity classes, potion refill identity and unsupported
tags. Tests were not run. Review was a brief static pass and diff whitespace
check only. No CI, version checks or merge were performed.

Other Cannon destinations, generic capabilities, Cannon firing, item automation,
Lantern visuals/light and destination comparator integration, redstone scheduling,
actual host execution, save/reload and LAN remain separate or unverified. The
fixture mocks world and storage behavior and is not an integration result.
