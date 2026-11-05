# Worn Dragonfall critical hits and safe falls

This addresses the high-priority #379 Dragonfall armor item. Source reference:
TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, `dragonfall.json`,
`AttributeModule`, `ModifierEvents.onCritical/onLivingFall`, `TinkerPredicate`,
`TinkerAttributes`, and Dragon Scale's shell-stat material trait.

## Critical hits

Each worn piece contributes `0.1 * level` using source float expression arithmetic
before widening. Contributions are added to the source critical attribute's 1.5
base and capped at 100; the boost is that bounded value minus 1.5. It adds to the
existing event multiplier, rather than multiplying damage by another percentage.

A positive boost also forces a critical hit while airborne: not grounded, not
climbing, not in water and not a passenger. Like source, this is not restricted
to descending, unsprinting or fully charged attacks. The native climbing registry
provides climbable-tag and trapdoor-above-matching-ladder behavior. Player flight
bypasses climbing, and the newer native glide-through exception is not adopted.
The read-only probe does not change the host's remembered climbable position.

An ordinary vanilla crit at one level becomes 1.6. A newly forced nonvanilla crit
starts at its original 1.0 and becomes 1.1, **not** 1.6. The hook runs before native
third-party critical callbacks, so they can still deny or change the result.
Existing TConstruct secondary AOE contexts are excluded, preserving their
noncritical damage handling. Native hit eligibility remains in the caller.

## Safe fall distance

Each intact worn piece contributes `2 * level`; the combined source bonus caps
at 100. The bonus is added as one transient native `minecraft:safe_fall_distance`
modifier. The native fall-damage consumer reads it, reducing ordinary fall damage
through the real threshold rather than merely changing a display or tooltip.
Equipment updates and recipe reload refresh/remove the modifier.

The source module types intentionally differ: critical uses percent-style
equipment-change hooks and remains on broken worn pieces; safe-fall uses the
normal item-attribute hook and is disabled when broken. Unknown durability is
conservatively excluded from safe-fall effects.

## Eligibility and limits

Both paths require current material/fixed traits plus integer upgrades, the live
worn-armor tag, correct stock armor class slot, and the complete known source
modifier definition. Malformed count/level, opaque wire patches and unsupported
resource state fail closed. No new persisted state is introduced.

Held Dragonfall, custom armor/modules, custom critical attribute commands,
Dragon Scale acquisition proof and arbitrary source armor-stat contributions are
not implemented. Remote critical prediction and actual guest/save/reload/LAN
behavior are untested. Ordering with other critical/attribute mods is not claimed
to reproduce Forge event priority.

Safe fall is a bounded native-threshold integration, not a replacement Forge fall
event. The host's floor/epsilon calculation, impulse-limited falls, block-specific
handling, external threshold multipliers and rounding remain native and can
differ from source's float distance reduction. Exact fall-event parity is deferred.

Brief static review and formatting only. Three pure regression tests were added
for critical arithmetic, forced-crit base values, caps and definition gates, but
were not run. No tests, builds, planner, CI, version checks or merges were run.
