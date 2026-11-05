# Production Melter fuel-driver regression

Run `HORIZON_HOST=/path/to/clean/pinned/horizon tconstruct/tests/run_melter_fuel_rust.sh` with cached Rust dependencies. The runner requires the clean tracked Horizon `0c7b5b61` tree, compares resolved dependencies with its lockfile, and checks that all seven named cases actually ran. The Mixins workflow runs this after the client check.

Unlike the bounded helper chain in #669, these tests execute the **unaltered production `tick_melter` body**, extracted at run time from `source-machine-runtime.rs`. They also extract its `kind`, `same_material`, `solid_fuel_rule` and tank-capacity helpers. Marker counts must be unique; no copied tick implementation or test-only scheduling algorithm substitutes for the production driver.

The source recipe archive, production Catalog/parser, Melter/Tank state transitions and the pinned host item/NBT codec are used directly. Tests run each tick through saved NBT and the separate live inventory. They cover:

- Four Copper Ingots producing the 360 mB required by the existing Controller casting route, with exact four-phase elapsed ticks, consumed lava packets and remaining fuel. This does not re-test Controller casting itself.
- Coal consumption and solid heat scheduling without draining the neighboring liquid fixture.
- Lava Bucket heater fuel and exactly one empty Bucket remainder, published only after a successful save.
- Rejected fuel-loading and completed-output writes preserving all saved states, live inputs, drops and lit notifications.
- No recipe, insufficient fuel temperature, a three-tick fuel item, completed-but-full output and an absent supporting structure.

## Explicit boundaries

This is a production-driver test with a **mock Game/world/store**, not a complete host playthrough. The store adapter supplies an all-or-nothing `write_many` and injected rejection; it does not prove the real store's world/region persistence or atomicity. The native inventory and NBT types are real, but block discovery, block entities, burn-time provider, item registry, recipe/tag snapshot wiring, light updates and spill delivery are adapters. The recipe wrapper supplies minimal known registries/tags, rejects recipe conditions, and declines damaged/material recipes; ordinary Copper Ingots are the tested input. GUI heat codes are stubbed and not asserted. Bucket-casting calls panic if unexpectedly reached.

The burn-time adapter supplies the conventional Coal 1600 and Lava Bucket 20000 values. These tests verify the production driver's division by four and remainder handling, not the host fuel provider or its overrides. Source solid fuel temperature/rate and liquid fuel amount/duration/temperature/rate come from the actual archive.

No claim is made about Smeltery/Foundry structure or fuel-cost scheduling, Faucet transfers, Controller texture metadata, recipe reload, real region storage, LAN, rendering or manual gameplay. No production behavior, recipes or acquisition routes change.
