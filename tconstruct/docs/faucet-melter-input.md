# Faucet input into Seared Melters

Issue #379, high-priority bounded fluid-handler expansion.

The fixed upstream [MelterBlockEntity](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/controller/MelterBlockEntity.java) exposes its ordinary fluid tank through `FLUID_HANDLER` without a face restriction. This adds filling that tank from all eight built-in Seared/Scorched fuel and ingot tanks/gauges, from formed Smeltery/Foundry Drains, from Casting Tanks, and from other Seared Melters, through a Faucet.

- Keeps the 2,160 mB Melter capacity, fluid/tag identity, 90 mB reservations, and immediate first and subsequent 10 mB pours.
- Stages source, destination, and Faucet saves together. Changes only the Melter tank, preserving raw item slots, fuel, heat, progress, and unrelated machine fields. Live inventory slots are not serialized or replaced.
- Requires readable canonical source and destination tank data and lossless Faucet data. Checks loaded chunks, distinct positions, expected blocks, entity presence, and raw snapshots before saving. Opaque tank state waits.
- Supports untagged fluid and registered vanilla base-potion identity. Custom potion color/effects and other tagged fluid wait, including in local play.
- Drain input validates the current owner, formed structure, shell position, controller block/entity, loaded chunks, and canonical tank before reserving. It takes only the selected exact fluid/tag layer and patches only the controller tank, preserving its raw slots, fuel, progress, and extensions. Ownership and structure are checked again before the shared commit. Invalid or unformed Drains wait.
- Melter sources require a readable source entity and canonical source tank. Only that source tank is patched; raw slots, fuel, progress, and extension fields in both Melters are preserved, and neither live inventory is rewritten. Self-transfers and other position aliases are refused.
- Casting Tank sources require canonical saved state matching both live slots and no retained wire backing. After a reservation, container reprocessing is staged with the source fluid, Melter, and Faucet; source live slots and all relevant snapshots are checked again after that callback. Live slots and Casting Tank comparator notifications update only after save success.
- A reserved packet can finish when a loaded source is removed. Unreadable supported sources wait. Stop finishes the current packet without reserving again.

This is not a generic external capability implementation. Filtered Duct and third-party sources into this destination remain outside this route. Melter comparator output integration, redstone scheduling, host/world execution, reload, and LAN remain unverified or separate work.

Fifteen production-route fixture cases and a wiring check were added (five tank-input, three Drain-input, four Casting Tank-input, and three Melter-input cases). They have **not been run**. Review was limited to static inspection and `git diff --check`; no tests, CI, version checks, or merge were performed.

The formed-Drain extension is stacked on PR #779. Its three added cases cover both controller/Drain variants, metadata and unselected-layer preservation, immediate packet timing, owner/structure/entity/save failures, Duct refusal, and exact base-potion selection changes. These cases use a mock structure detector, not real world formation.

The Casting Tank extension is stacked on PR #780/#779. Its four added cases cover bucket reprocessing with saved/live slots and comparator notifications, save and post-processing snapshot failures, opaque/stale/full-state refusal, and base-potion packets that outlive a source-container refill. Container processing uses the production state method with fixture registry predicates; real host datapack reload and LAN are not validated.

The Melter-to-Melter extension is stacked on PR #785/#780/#779. Its three added cases cover raw metadata/live inventory preservation, first and subsequent packet timing, failed saves and unreadable source refusal, aliases, exact potion identity across a source refill, and the final 5 mB of destination capacity.
