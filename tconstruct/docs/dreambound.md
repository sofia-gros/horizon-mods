# Worn Dreambound gravity

This is the gravity portion of the high-priority #379 Dreambound item.
Extra jumps use the shared movement bridge described below. Double Jump
combination is documented in [Double Jump runtime](double-jump-runtime.md).

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

## Shared extra-jump runtime

Intact worn Dreambound adds one airborne jump per positive-level piece. It is
independent of the exact-level gravity condition and does not scale with level.
Double Jump adds its logical level to the same allowance. The combined source
attribute has base one and maximum 100, so there are at most 99 extra jumps.

The existing bridge calls the real native jump routine on a fresh input edge,
tracks one actor-owned spent budget, and resets it on ground jumps/landings.
Equipment/effect refresh does not refill an airborne budget. Permission frames
are bound to all four current equipment stacks; the authoritative guest request
recomputes eligibility. Prediction-recording servers reject this path rather
than changing or bypassing their replay validation. Source sound/particles,
negative-gravity jumping, held items and full runtime validation remain outside
this bridge. No duplicate jump loop or independent Double Jump budget is added.

## Remaining limits and review

Held/custom armor, custom definitions, Phantom-shell acquisition proof, Forge's
negative-gravity semantics and exact cross-engine attribute ordering remain
outside this change. Actual equipment transitions, save/reload, remote attribute
synchronization, guest swaps and LAN have not been exercised.

Brief static review and formatting only. The pure rule and wire tests remain
unrun. No tests, builds, planner, CI, version checks or merges were run.

Static integration correction: jump capacity, spent count and pending request live
in owned `StatusFlags` fields. Its explicit `PlayerEffects::status` construction
is supported by the mixin constructor validator, unlike the fallible
`PlayerMotion::with_attribute_identities` body. The ordinary `apply_effect_status`
local initialization preserves these three fields from the same actor before
replacing effect flags, so refreshing effects cannot refill an airborne budget.
