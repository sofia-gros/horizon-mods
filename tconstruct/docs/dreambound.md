# Worn Dreambound gravity

This is the gravity portion of the high-priority #379 Dreambound item.
**Extra jumps are not implemented by this patch.**

Source: TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, generated
`dreambound.json`, `ModifierProvider`, `ModifierCondition`, `AttributeModule`,
and Phantom's shell-stat material trait.

## Implemented

The source gravity module is a flat `0.15f` MULTIPLY_TOTAL modifier conditioned
on **exact logical level 1**. The provider explicitly uses `exactLevel(1)`.
It is +15% gravity per eligible piece, not reduced gravity and not +15% per level.
Level 0 and level 2+ have no gravity contribution. Separate equipped pieces
multiply independently through the native attribute engine.

The runtime resolves live material/fixed traits and integer upgrades on stock
worn armor in its correct class slot. It requires the live worn tag and complete
known source modifier definition; malformed count/level, opaque wire patches and
missing snapshots fail closed. The percent-style equipment-change module has
no brokenness filter, so broken worn pieces retain the gravity effect.

Owned transient per-slot `minecraft:gravity` modifiers are refreshed after
equipment changes and resource snapshot publication. Unequip, moving the item
to a wrong slot, changing its level away from one, or invalidating the definition
removes the bonus. Native movement reads the actual gravity attribute, so the
change composes with existing Heavy/Vintage/Solid and native movement effects.
No saved derived value or jump counter is added.

## Extra-jump work still required

Source Dreambound also supplies a flat +1 jump-count attribute per intact piece,
independent of its level. `ToolClientEvents.handleKeyBindings` detects fresh jump
presses, runs `DoubleJumpHandler.extraJump` locally, then sends a DOUBLE_JUMP
control packet for the authoritative entity. The shared routine checks ground,
climbing and water/bubble state, consumes the airborne counter, invokes the real
jump routine, and emits sound/particles. Ground jumps and landings reset usage.

The current native `Game::native_movement_input` hook exposes only `&self`;
the actual `MotionWorld::jump_from_ground` has no live equipment context. Adding
a velocity shortcut at either boundary would omit authoritative counter and
client-request handling. A dedicated bridge must carry the resolved allowance,
validate fresh-press requests on the acting server player, maintain/reset usage,
and synchronize prediction, sound and particles. No such bridge or substitute
jump-count grant is claimed here.

## Remaining limits and review

Held/custom armor, custom definitions, Phantom-shell acquisition proof, Forge's
negative-gravity semantics and exact cross-engine attribute ordering remain
outside this change. Actual equipment transitions, save/reload, remote attribute
synchronization, guest swaps and LAN have not been exercised.

Brief static review and formatting only. Two pure regression tests were added
for exact-level gating, float widening and definition rejection, but were not
run. No tests, builds, planner, CI, version checks or merges were run.
