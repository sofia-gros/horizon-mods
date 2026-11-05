# Overburn recovery candidate

This candidate implements player-carried Overburn after the Overgrowth work.
It is not yet a validated release feature. It uses the source tank capacity and
container-transfer paths already present in the port.

- Each holder has an independent transient interval: levels 1–6 recover every
  160, 80, 40, 20, 10 or 5 authoritative inventory ticks. Higher levels use 5.
- Main inventory, worn armor and offhand are included; the actively used stack
  is skipped. Full or inactive Overslime and invalid data consume no fuel.
- A supported world melting-fuel recipe supplies amount, duration and rate.
  Less than one recipe consumes only the remaining fluid and scales duration
  by that fraction, after multiplying by the level's interval and dividing by 5.
- Fuel expiration and rate are saved in `tic_persistent` under
  `tconstruct:overburn`. Expiration is inclusive. A very short fill still grants
  the current recovery but is not saved if it expires before the next update.
- Each update restores rate / 10 points, with rate % 10 as a chance out of ten
  for one extra point. Overworked multiplies restoration by 1 + its level.
- Fluid, saved fuel and shield changes are staged on one copy before replacing
  the live stack. Other item data is preserved. Rebuilds without Overburn clear
  only its saved fuel; existing restrictions on trait-changing swaps remain.

The exact pinned Overburn definition is required on each recovery pass.
Missing/changed modifier definitions and unresolved snapshots fail closed.
Tagged fuel, non-player inventories and custom module definitions are not
supported. Cadence starts over with a newly created holder/world phase, unlike
saved fuel expiration, which uses absolute world game time. General source
fuel selection and datapack limitations of the existing world lookup remain.

Source: [OverburnModule at f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tools/modules/OverburnModule.java).

Six focused rule tests and three real ItemStack/tank planner cases passed
during implementation. The real ItemStack fixture is provided by `python3 tconstruct/tests/run_overburn.py --horizon PATH`.
Full Mixins application, current client build, broad suites, CI, actual
inventory timing, save/reload and LAN are deferred to the final validation pass.
