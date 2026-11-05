# Lantern Faucet supply into Fluid Cannon

Issue #379, high-priority bounded fluid-handler expansion. Stacked on PR #762/#730/#723.

Placed Seared and Scorched Lanterns can supply the Seared Fluid Cannon through a Faucet. A route-local adapter reads the canonical 50 mB Lantern tank; multiblock fuel classification and unrelated transfer routes remain unchanged. This follows the all-face fluid handler inherited by upstream LanternBlockEntity from TankBlockEntity.

The existing guarded path preserves the Cannon item and retained wire checks, source/Faucet/Cannon raw snapshots, loaded chunks and block/entity identity. Source, Faucet, and Cannon writes are staged together. It respects the 2,000 mB destination capacity, pours immediately after each reservation, and preserves the reserved packet's exact identity after a source refill. Untagged fluids and registered vanilla base-potion tags are accepted; custom potion color/effects and other unsupported tags wait, including locally.

Three fixture cases were added for both Lantern variants, packet timing and item preservation, failed saves/opaque/missing/stale/unloaded states, potion identity across refill, and the final 5 mB of capacity. They were **not run**. Review was limited to static inspection and `git diff --check`; no CI, version checks, or merge were performed.

This is fluid supply only, not Cannon firing or generic third-party capabilities. Lantern light/rendering/source comparator integration, redstone scheduling, actual host behavior, save/reload, and LAN remain separate or unverified.
