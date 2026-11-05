# Fluid Cannon supply to Casting Tables and Basins

Issue #379, stacked on #813 / #773 / #770 / #692. The placed Seared Fluid Cannon
can supply all four Seared/Scorched Casting Tables and Basins through a Faucet.
This adds fluid extraction, not Cannon firing or item automation.

FluidCannonBlockEntity inherits TankBlockEntity's all-face fluid handler at the
fixed upstream revision used by the port:
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/FluidCannonBlockEntity.java
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/component/TankBlockEntity.java

The guarded route requires canonical Cannon data and a matching live item,
refusing retained wire-backed items. It patches only the source tank and leaves
the live item untouched. Casting extension fields are preserved. Source,
Faucet and casting state commit together only after loaded chunk, block/entity,
raw/live item and fresh recipe checks. Source comparator notifications occur
after successful saves.

First and later reservations pour immediately and are limited by remaining
recipe capacity. Pending fluid keeps its identity after source refills and
waits when a recipe is stale. Supported-potion rules are inherited: registered
base potions can cross network contexts; supported custom tags are local-only
and still require a matching recipe. Opaque or missing state and failed saves
do not publish the staged transfer.

Four fixture cases plus wiring coverage were added for all four casting targets,
source item and destination extension preservation, packet timing and comparator
changes, eight refusal conditions, stale recipes and packet identity, stopping,
and a final 5 mB fill. Tests were not run. Review was a brief static pass and
diff whitespace check only. No CI, version checks or merge were performed.

Other Cannon output destinations, generic capabilities, firing, redstone
scheduling, actual host execution, save/reload, datapack reload and LAN remain
separate or unverified. Recipes, world and storage are mocked in the fixture.
