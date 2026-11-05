# Turtle's Grace: breathing contribution (#379)

This local slice connects Turtle's Grace's constant Respiration enchantment on
stock worn armor. The normal Turtle material's `shell` stat grants this trait.
The separate swim-speed attribute, held items, arbitrary additional armor types,
and out-of-slot native Respiration enchantments are outside this slice.

## Behavior

The exact canonical modifier definition, current material traits, independent
saved modifier levels, actual armor slot and resolved worn-armor tag must match.
Malformed item/source data removes the owned bonus. Unlike the attribute module,
source `EnchantmentModule.Constant` and `EnchantmentModifierHook` do not skip
broken tools: a broken Turtle shell retains its breathing contribution.

The strongest worn-piece Respiration result wins, rather than summing Turtle's
Grace copies. Hooks on the same helmet add before that maximum. The projection
adds only the difference above native helmet Respiration plus the existing TiC
head Respiration bonus. Other native oxygen modifiers remain unchanged. An
ordinary helmet with Respiration III therefore receives no extra benefit from a
separate Turtle's Grace I shell. This does not repair the prior Respiration
slice's broken-helmet limitation or project other virtual enchantment sources.

Equipment updates, server-player tick and source reload refresh both owned
bonuses in order. Native guest actor swaps select each guest's armor and
attributes; remote clients rely on authoritative attributes rather than adding
another local modifier. No native repository edits or fluid-movement dependency
are used. The provenance-blocked swim-speed draft is not incorporated.

## Sources and verification limits

TConstruct commit `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` (MIT, license read):

- `src/generated/resources/data/tconstruct/tinkering/modifiers/turtles_grace.json`
- `src/generated/resources/data/tconstruct/tinkering/materials/traits/turtle.json`
- `EnchantmentModule.Constant`, `EnchantmentModifierHook` and
  `ModifiableArmorItem` in the source library.

This is original adapter code implementing the documented module contract; no
upstream implementation is copied. Stack base: `feat/respiration-armor-379`
(`07d3e66e`). Publication remains paused.

Brief static review and `git diff --check` only. Arithmetic/source-definition
regression cases are included but unrun. No tests, builds, planner transforms,
CI checks, version checks, merges, live air-consumption observation, saving or
LAN verification were performed. Keep the scope partial until the final batch.
