# Remaining Protection acquisition

This #379 slice adds normal acquisition for Blast, Magic, Melee and Projectile
Protection, sharing the preceding Fire Protection defense-budget implementation.
This acquisition slice does not implement their damage or secondary effects.
The later [Projectile Protection attribute slice](projectile-protection-attribute.md)
adds its knockback-resistance effect and updates that UI notice; the damage
protection path remains separate.

Source reinforcement inputs are distinct:

- Blast Protection: Obsidian Reinforcement.
- Magic Protection: Gold Reinforcement.
- Melee Protection: Cobalt Reinforcement.
- Projectile Protection: Iron Reinforcement.
- Existing Fire Protection: Seared Reinforcement.

Each uses one unit per reinforcement and five units per level. Starting a
partial level occupies one defense slot. Completing that partial progress uses
its existing slot. A crystal adds five units and requires one more defense slot.
Extraction removes five units and preserves any remaining partial progress.
All five Protection types and Knockback Resistance share the same defense
budget; their amounts are excluded from upgrade and ability usage. Fortified
and material-swap capacity checks include all these defense entries.

Current armor/held tags, the preceding slice's 13 worn items and three shields,
normal source snapshot/rebuild transactions, malformed-progress rejection and
the planner ceiling of 32767 levels remain unchanged. Other held tools are not
added. Existing extraction selection indices are preserved. The exact source
recipe and each modifier's distinct highest definition are required.

## Runtime remains separate

The definition contracts preserve meaningful differences rather than assuming
these are vanilla enchantments:

- Blast: 3 protection points plus non-protection Blast Protection enchantment
  behavior, such as explosion knockback reduction.
- Magic: 3 protection points plus a maximum-armor bad-effect-duration multiplier
  of -0.05 per level.
- Melee: 2 protection points for the source melee tag and direct damage only,
  plus maximum-armor use-item speed of +0.05 per level.
- Projectile: 2 protection points plus maximum-armor knockback resistance of
  +0.05 per level.

Those runtime effects, maximum-versus-summed attribute aggregation, held
activation, fractional effective levels, source protection caps and live damage
tags still need a separate damage/attribute bridge. No substitutes are installed
here. Acquisition does not complete the full Protection checklist item.

Brief static review covered input mapping, recipe/definition differences,
shared defense accounting, partial progress, crystal/extraction and material
swap checks. Regression cases were added but not run. No tests, builds, Mixins
transforms, CI inspection, version checks or merge were performed. Gameplay,
save/reload and LAN remain unverified. Publication is paused. This branch stacks
on Fire Protection acquisition and its Fortified/Knockback prerequisites.

Sources: Tinkers' Construct f4e77d43, the four corresponding
`recipes/tools/modifiers/defense/*_protection.json` and
`tinkering/modifiers/*_protection.json` resources.
