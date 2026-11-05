# Bounded survival helper-chain regression

Run from any directory with the pinned Horizon checkout and its cached Cargo dependencies:

```sh
HORIZON_HOST=/path/to/clean/horizon tconstruct/tests/run_survival_chain_rust.sh
```

The runner verifies Horizon `0c7b5b61a5e746d621afffba919b12aeccf9c0fa`, rejects tracked modifications, and checks every resolved dependency against that host's lockfile. It extracts the bundled fixed-source recipe archive into a disposable directory, runs the two named cases, and rejects a successful zero-test filter. The main Mixins workflow runs it after host dependencies have been populated by the client check.

## Executed production helpers

- Catalog selection from the complete bundled recipe archive, using an explicitly constructed minimal item/fluid/tag snapshot.
- Melter heat steps and item decrement: four copper ingots yield 360 mB for a Controller, and fourteen copper plus fourteen gold ingots yield 2520 mB for the alloy branch.
- Controller Casting state: one Seared Bricks input and 360 mB copper are consumed only after complete filling and cooling.
- SmelteryTank atomic alloy operations in balanced batches, followed by actual Casting state for three 810 mB Rose Gold Blocks. The final tank retains 90 mB; all 28 metal ingots are consumed.
- Production Melter, Casting and tank NBT encoding/decoding, including every heat/cooling step, and the real host item codec for resulting blocks.
- Source lava fuel parsing (50 mB, duration 100, temperature 1000, rate 10), one Tank deduction and its persisted 950 mB remainder from a 1000 mB starting tank.
- Rejection without consumption below copper melting temperature, below rose-gold alloy temperature, with missing alloy input, with a partially filled cast, and after invalidating a filled cast.
- Shipped native Anvil recipe costs: three resulting Rose Gold Blocks and four brick-tag positions. This is a recipe-cost assertion, not execution of crafting.

## Boundaries

This is not a full Game, world-tick or natural-resource playthrough. The fixture supplies registry/tag snapshots, accepts recipe conditions, schedules heat calls and manually transfers fluid between helpers. It uses the Melter heat helper for both metal batches; it does not execute Smeltery inventory/heat scheduling, structure discovery, Furnace/Heater solid-fuel consumption, Faucet scheduling or cross-block atomic commits.

The fuel check tests one container deduction and persistence only. `Melter::heat_slot` does not consume fuel; the fixture explicitly asserts that its stored fuel remains unchanged. It does not predict the route's fuel budget, duration, or prove the world's fuel driver. NBT round-trips are in-memory codecs, not world storage, reload, recovery or opaque-state quarantine. Controller texture metadata and actual Anvil crafting/consumption, workstation assembly/repair, GUI, multiple viewers, LAN and gameplay remain untested here.

The fixture reuses production-module imports from the existing Seared Reinforcement harness. Its potion-name and portable-capacity stubs are not used by these two cases. Other imported module tests are filtered out rather than counted as newly executed coverage. No production recipes, machine behavior or acquisition routes change.
