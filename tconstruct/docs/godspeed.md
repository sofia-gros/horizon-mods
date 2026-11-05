# Worn Godspeed attributes

This addresses the high-priority #379 Godspeed armor item, stacked on Cobalamin
PR #742 so both modifiers share one attack/mining integration. It does not
implement Kinetic, which is a separate on-damage effect.

Pinned source: TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, generated
`godspeed.json`, `AttributeModule`, `EquipmentChangeWatcher` and
`ToolEvents.onBreakSpeed`. Source Ichor's slime-stat trait and Ichorskin's default
trait both grant Godspeed at level 1.

For each positive-level piece, apply these independent MULTIPLY_TOTAL modifiers:

- Attack speed: `0.01 + 0.015 * level` (2.5% at level 1).
- Movement speed: `0.025 + 0.025 * level` (5% at level 1).
- Mining-speed attribute: `0.025 + 0.05 * level` (7.5% at level 1).

Flat terms apply once per contributing equipment slot, not once per level and
not to empty/zero-level slots. The source expression uses float arithmetic before
widening to double. Worn Cobalamin and Godspeed combine multiplicatively, with
attack/mining caps of 1024/100 and a single final float cast for mining speed.
The native movement attribute engine supplies movement's operation ordering and
bounds, including composition with existing Speedy, potion and sprint modifiers.

## Coverage and cleanup

The shared runtime resolves live material/fixed traits and integer upgrades on
stock armor in its correct class slot, requiring the live worn-armor tag and the
known complete source definition. Resource/definition failures are isolated per
modifier; changing Cobalamin's definition does not disable known Godspeed.
Malformed levels/counts and opaque wire patches fail closed.

Percent-style source AttributeModule uses equipment-change hooks with no broken
check, so broken worn pieces retain these effects. Actual native and TConstruct
main-hand cooldown consumers receive the attack multiplier. Mining includes
native items, bare hands and conditional tool additions. The movement modifier is
attached to the real player attribute consumed by native movement and sweep
speed checks, not a display-only or saved tool-stat bonus.

Own transient per-slot attack/movement modifiers refresh on equipment changes
and recipe-snapshot publication, and are removed when absent or unsupported.
The runtime reuses #742's hooks and final mining bridge rather than introducing
another separately rounded mining hook. No new persisted state is introduced.

## Deferred

Held Godspeed, custom armor/module support, acquisition proof, custom mining
attribute commands and arbitrary external attack/mining attributes remain out of
scope. The existing chest-Haste baseline is already rounded to float. Full
cross-mod attribute ordering/rounding is not claimed. Native movement uses the
host's attribute precision and caps rather than a new Forge attribute engine.

Only authoritative local/server state is resolved here. Remote attack/mining
prediction needs an actor-specific bridge; native movement attribute networking,
guest swaps, real save/reload, equipment transitions and LAN have not been tested.

Brief static review and formatting only. Two pure regression tests were added
for flat/level math, zero-level slots, products, caps and source-definition gates.
No tests, builds, planner, CI, version checks or merges were run for this change.
