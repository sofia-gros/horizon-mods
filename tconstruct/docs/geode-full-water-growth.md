# Budding slime crystal growth in full water

TConstruct f4e77d43 `BuddingCrystalBlock.randomTick` delegates new-bud
placement to vanilla `BuddingAmethystBlock.canClusterGrowAtState`: air, or a
water block with fluid amount 8. It separately sets the new bud's waterlogged
property when the old fluid's type is still water. The four custom budding
blocks remain the only targets of this adapter; vanilla budding amethyst is
unchanged. Existing same-kind buds still advance regardless of facing, as in
the TConstruct override, and retain the source's direction and water type rule.

A previously prepared local Horizon resource snapshot exposes water level 0 as
Water/amount 8/not falling, levels 1–7 as FlowingWater/amount 7–1, and levels
8–15 as FlowingWater/amount 8/falling. The former code's exact equality to
`FluidState::WATER` excluded those full falling states. The new condition
tests block identity and full amount for initial growth, while testing
`FluidKind::Water` separately for waterlogging. The snapshot's exact source
revision is not established; it is supporting evidence, not an integrated
target-host run.

Standalone tests cover air, full and shallow water, unrelated blocks, the
existing four kinds and their growth stages. A small Rust harness also uses the
host's real FluidState types for source and full-falling water. The adapter
wiring has a static test. Random-tick dispatch, fluid state export, world
placement, update behavior, save/reload, game display and LAN remain unverified
on the target build.
