# Fire Protection acquisition

This #379 slice adds source Fire Protection acquisition, not its damage or
burn-duration effects. The workstation description explicitly states that the
runtime effects are not connected yet.

Each Seared Reinforcement adds one of five units for a level. Starting a level
occupies one defense slot; completing its existing partial progress needs no
additional slot. One operation fills at most the current level. A matching
Modifier Crystal adds five units and therefore needs one additional defense
slot, even when the tool already has a partial level.

Current armor or held tags must admit one of the supported 13 worn armor items
or Travelers/Plate shields/Battlesign. Other source-held tools are outside this
bounded slice. Upgrade and ability slots are not charged. The shared defense
budget includes both Fire Protection and Knockback Resistance; admitting either
cannot overwrite the other's occupied count. Existing Fortified capacity and
material-swap checks include partial Fire Protection levels.

Extraction removes five purchased units and retains partial progress. A
partial level below five units is not extractable. Existing selection indices
are preserved. The current world's exact source recipe and highest modifier
definition are required. Malformed progress is rejected. Source specifies no
recipe level cap; this implementation uses the planner's explicit 32767-level
representation ceiling (163835 units), not an invented gameplay max of five.

## Runtime boundary

Source Fire Protection adds 3 protection points per effective level against
its live `tconstruct:protection/fire` damage tag, after the source can-protect
check. The stock tag includes fire, lightning and Shock. A separate enchantment
module supplies Fire Protection's non-protection effect, notably burn-duration
reduction, while suppressing vanilla's protection contribution. Substituting a
vanilla Fire Protection enchantment would not match those semantics.

No such substitute, early incoming-damage multiplier or glider/armor attribute
is installed here. Post-armor protection aggregation, fractional progress,
protection caps, secondary enchantment effects, held activation and damage-tag
reload remain separate runtime work. This slice does not complete the full
Fire Protection checklist item or the other Protection modifiers.

Brief static review covered five-unit progress, shared defense budgets, crystal
capacity, extraction, Fortified/material swaps and override rejection. Regression
cases were added but not run. No tests, builds, Mixins transforms, CI inspection,
version checks or merge were performed. Workstation behavior, save/reload and
LAN remain unverified. Publication is paused. This branch stacks on Fortified
and Knockback Resistance, independently of the other acquisition/melee chains.

Sources: Tinkers' Construct f4e77d43,
`recipes/tools/modifiers/defense/fire_protection.json`,
`tinkering/modifiers/fire_protection.json` and
`tags/damage_type/protection/fire.json`.

The later [explicit-ignition duration bridge](fire-protection-duration.md) adds
the secondary effect for player `ignite_for_ticks` callers. Standing-in-fire
contact and damage protection remain outside that bridge.
