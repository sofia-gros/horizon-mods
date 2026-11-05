# Placed Lantern supply to Casting Tables and Basins

Issue #379, stacked on #773 / #770 / #692. Both placed Seared and Scorched
Lanterns can supply all four Seared/Scorched Tables and Basins through a Faucet.
This uses the existing guarded machine-to-casting route and does not classify
Lanterns as multiblock fuel tanks or activate unrelated routes.

The source is a canonical 50 mB tank. The route reserves only the available
fluid and remaining recipe space, pours the first 10 mB immediately, and keeps
reserved fluid separate from later Lantern refills. Casting recipes are
resolved and checked again before each commit. Partial casts can be continued
with later fills; a stale recipe waits without discarding a packet.

Source, casting state, and Faucet are staged and saved together. Casting
extension fields survive. Unreadable or noncanonical Lantern state, changed
live casting items, missing block entities, unloaded chunks, changed blocks or
raw snapshots, and failed saves refuse the transfer. Source and destination
changed notifications follow successful saves. Existing supported-potion rules
remain: only exact registered vanilla base potion tags in network contexts;
supported custom potion data remains local-only and needs a matching recipe.

Source basis: LanternBlockEntity inherits TankBlockEntity's all-face fluid
handler at the fixed source revision already used by the port:
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/LanternBlockEntity.java
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/component/TankBlockEntity.java

Four fixture cases plus wiring coverage were added for the eight source/target
pairs, refill timing, preserved casting extensions, seven refusal conditions,
stale recipes and packet identity after refill, stopping, and a final 5 mB fill.
Tests were not run. Review was limited to a brief static pass and diff whitespace
check. No CI, version checks, or merge were performed.

This is fluid supply, not complete casting or Lantern parity. Generic
capabilities, Lantern rendering/world light/source comparator integration,
redstone scheduling, and actual host, save/reload, datapack reload and LAN
behavior remain separate or unverified. The fixture mocks recipe/world/storage
behavior and does not establish those integration properties.
