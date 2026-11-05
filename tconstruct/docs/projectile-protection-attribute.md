# Projectile Protection knockback resistance

This #379 slice connects Projectile Protection's secondary maximum-armor
attribute, following its normal acquisition implementation. It does not yet
connect the separate projectile-damage protection module.

The actor receives +0.05 knockback resistance per **highest effective level**
among eligible equipped pieces. Levels are not summed across armor or hands.
A one-unit partial upgrade uses the source incremental f32 level correction,
not a whole level; material traits and purchased five-unit progress are combined
once. This native attribute affects knockback generally, not only projectile
hits. Existing armor stats and other independent native attribute modifiers
are retained.

Supported equipment is the 13 known worn armor items in their actual class
slots, plus Travelers/Plate shields and Battlesign in either hand. Armor must
retain its current worn-armor tag and held shields their current held tag.
Held shields need not be actively blocking. Other source-held tools remain
outside this bounded bridge. Unknown, broken, malformed or unresolved stacks
do not contribute, and the exact highest source modifier definition is required.

The owned modifier is transient. Equipment updates, player ticks and resource
reload recompute it from current stacks, traits and tags. Removal, breaking or
loss of eligibility lowers it to the next maximum or removes it entirely.
The host's actor swap supplies the current guest's equipment. Remote clients
do not add a second copy to server-synchronized attributes. No extra level is
written into tools or native enchantments.

## Remaining boundaries

Projectile damage reduction, source damage-tag matching, protection caps,
other maximum-attribute modules sharing the same source data key, arbitrary
effective-level hooks, additional held tools, custom equipment classes and
monster equipment are not implemented here. Armor refresh occurs at the
existing equipment/tick boundaries rather than claiming every Java event's
exact timing. The previous acquisition document's blanket runtime limitation
is superseded only for this Projectile Protection knockback attribute.

The separate use-item-speed work already contains a Melee Protection secondary
bridge; its integer-level assumptions must be reconciled with five-unit saved
progress when integrating the acquisition changes. It is not copied into this
branch and must not be treated as universally absent.

Brief static review covered the maximum-not-sum rule, partial-level arithmetic,
source slot eligibility, owned modifier removal and native attribute naming.
Pure formula regressions were added but not run. No tests, builds, Mixins
transforms, CI inspection, version checks or merge were performed. Actual
knockback, save/reload and LAN remain unverified. Publication is paused.

Sources: Tinkers' Construct f4e77d43, `projectile_protection.json`,
`MaxArmorAttributeModule.java`, `MaxArmorLevelModule.java` and
`ArmorLevelModule.validSlot`.
