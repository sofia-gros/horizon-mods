# Placed Lantern supply through formed Drains

Issue #379, stacked on #747 / #740 / #736. Both placed Seared and Scorched
Lanterns can supply a formed Smeltery or Foundry through a Seared/Scorched Drain
and Faucet. The adapter is local to the guarded input route; Lanterns remain
excluded from multiblock fuel-tank classification.

The Lantern must expose a canonical 50 mB tank and a loaded block entity.
Source, Faucet and controller changes are staged together. Controller metadata,
items and unrelated fluid layers are preserved by patching only its tank.
An offered fluid joins only the exact matching fluid/tag layer. Capacity comes
from the formed structure, which must still match the saved tank capacity.

Reservations cannot exceed source fluid or free total controller capacity. The
first 10 mB pours immediately, including after later reservations. A reserved
packet keeps its own identity if the Lantern is refilled with another fluid.
Untagged fluids and exact registered vanilla base potions are supported. Custom
potion colors/effects are refused on this route even locally. Opaque or missing
recognized source state, stale structure ownership/capacity, unloaded chunks,
changed blocks or raw snapshots, and failed saves wait without publishing the
staged transfer. A loaded removed source can finish its already reserved packet,
as in the existing route.

The source compatibility follows LanternBlockEntity's inherited all-face
TankBlockEntity fluid handler at the fixed source revision used by the port:
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/LanternBlockEntity.java
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/component/TankBlockEntity.java

Four fixture cases and wiring coverage were added for both Lantern/controller
pairs, controller metadata and layer preservation, first/later packet timing,
eight refusal conditions, distinct base potion layers after refill, final 5 mB
capacity, and local/network custom-potion refusal. Tests were not run. Review
was a brief static pass and diff whitespace check only. No CI, version checks,
or merge were performed.

Generic capabilities, Duct input/filter behavior, Lantern rendering/world light
and source comparator integration, redstone scheduling, actual host execution,
save/reload and LAN remain separate or unverified. The fixture mocks world,
structure and storage behavior; it does not establish those integrations.
