# Faucet input into Seared Melters

Issue #379, high-priority bounded fluid-handler expansion.

The fixed upstream [MelterBlockEntity](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/controller/MelterBlockEntity.java) exposes its ordinary fluid tank through `FLUID_HANDLER` without a face restriction. This adds filling that tank from all eight built-in Seared/Scorched fuel and ingot tanks/gauges through a Faucet.

- Keeps the 2,160 mB Melter capacity, fluid/tag identity, 90 mB reservations, and immediate first and subsequent 10 mB pours.
- Stages source, destination, and Faucet saves together. Changes only the Melter tank, preserving raw item slots, fuel, heat, progress, and unrelated machine fields. Live inventory slots are not serialized or replaced.
- Requires readable canonical source and destination tank data and lossless Faucet data. Checks loaded chunks, distinct positions, expected blocks, entity presence, and raw snapshots before saving. Opaque tank state waits.
- Supports untagged fluid and registered vanilla base-potion identity. Custom potion color/effects and other tagged fluid wait, including in local play.
- A reserved packet can finish when a loaded source is removed. Unreadable supported sources wait. Stop finishes the current packet without reserving again.

This is not a generic external capability implementation. Melter, Casting Tank, Drain/Duct, and third-party sources into this destination remain outside this route. Comparator output integration, redstone scheduling, host/world execution, reload, and LAN remain unverified or separate work.

Five production-route fixture cases and a wiring check were added. They have **not been run**. Review was limited to static inspection and `git diff --check`; no tests, CI, version checks, or merge were performed.
