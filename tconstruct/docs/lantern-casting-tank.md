# Lantern Faucet supply into Casting Tanks

Issue #379, high-priority bounded fluid-handler expansion. Stacked on PR #766/#728.

Placed Seared and Scorched Lanterns can supply a Casting Tank through a Faucet. Upstream LanternBlockEntity inherits TankBlockEntity's all-face fluid handler; this route uses a local adapter for the canonical 50 mB source, without classifying Lanterns as multiblock fuel tanks or changing unrelated routes.

The existing staged path reserves at most the available source and destination capacity, pours 10 mB immediately, and processes the destination container on the staged state. It validates readable canonical source data, saved/live destination slots, retained wire backing, loaded chunks, block/entity identity, and raw snapshots before committing. Source, Faucet, and Casting Tank saves are atomic; destination live slots and notifications update afterward. Noncanonical or uncertain state waits.

Registered vanilla base potions are supported with exact tag identity. Supported custom color/effects remain local-only and are refused in network contexts. A reserved packet retains its own identity after a source refill. This does not add generic third-party capabilities, Lantern rendering/world light/comparator integration, or redstone scheduling.

Three fixture cases and a wiring check were added for both Lantern variants, 50 mB conservation and bucket completion, failed saves/opaque source/post-processing races, and base/local-custom/network-custom potion behavior. The tests were **not run**. Review was limited to static inspection and `git diff --check`; no CI, version checks, or merge were performed. Actual host/datapack reload, world save/reload, and LAN remain unverified.

Sources:
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/LanternBlockEntity.java
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/component/TankBlockEntity.java
