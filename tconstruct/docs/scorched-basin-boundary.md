# Scorched Basin subset

Pinned source: TConstruct f4e77d43f39ba1047bf93ae816b7e1beaab514c2. The Scorched Basin uses the same Basin block entity, shape and recipe-driven tank capacity as the Seared Basin, but requires a physical cast. This implementation routes it through the existing basin machinery, including persistence, hopper, faucet and drop/spill paths. Those paths retain their existing host limitations; this is not a game-verified result.

The source recipe is `# # / #G# / ###`, where # is scorched brick and G is gold platform. It is deliberately not installed: gold platform is unregistered. An unresolved item in a vanilla crafting recipe is unsafe; no substitute ingredient is equivalent. See f11-state-boundaries.md for the missing six-direction connection, placement, waterlogging, neighbor-update and 64-shape support. Consequently ordinary survival acquisition is **not** implemented. The registered basin appears in the TConstruct Creative blocks tab and can also be obtained by command; recipes using a real input can work without a gold platform, while empty-cast recipes cannot.

The item uses the pinned covered model. The host template does not expose source's `has_item` state, so the placed static model uses the open basin model, as the current Scorched Table similarly does. Visual state parity remains open.

The held-container and Faucet-resume paths from #560 recognize this basin as a
casting target, including retaining a reserved packet when its state is
unreadable. Their existing conservative metadata restrictions still apply.
Tests cover state transitions and static routing; actual host registration,
save/drop behavior, hopper, held-container transfers, and LAN remain unverified.
