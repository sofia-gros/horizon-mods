# Faucet transfers between Fluid Cannons

Issue #379, stacked on #826. A placed Seared Fluid Cannon can supply another
Seared Fluid Cannon through a Faucet. This route is dispatched before legacy
fallbacks and does not alter other source/destination pairs. It transfers fluid,
not items, and does not implement Cannon firing.

Both Cannons require canonical saved state, identical live items and no retained
wire backing. Only each nested tank is patched. Neither live inventory is
rewritten. Source, Faucet and destination save together after block/entity,
loaded-chunk, raw and live-item rechecks. Comparator and changed notifications
follow successful saves.

The 2,000 mB destination capacity, exact fluid/tag identity and reserved packet
ownership are preserved. First and later reservations pour immediately. Untagged
fluids and exact registered vanilla base potions are supported; custom potion
data waits even locally. A pending packet can finish after an unsupported source
refill, but that source cannot reserve another packet. Opaque or missing state,
self-transfers and failed saves refuse staged changes.

Source basis is the inherited all-face fluid handler at the fixed port revision:
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/FluidCannonBlockEntity.java
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/component/TankBlockEntity.java

Four focused fixture cases plus wiring coverage were added for both inventories,
packet timing and notifications, failure states on either participant, self-transfer,
exact potion identity and final 5 mB capacity, pending identity and stopping.
Tests were not run. Review was a brief static pass plus diff whitespace check.
No CI, version checks or merge were performed. Actual host execution, save/reload,
LAN, redstone scheduling, firing and generic capabilities remain unverified or
separate. The fixture mocks world and storage behavior.
