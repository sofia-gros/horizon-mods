# Worn Balm of Ssssss effect duration

This addresses the high-priority #379 Balm of Ssssss harmful-duration item. It is
stacked on Crystalstrike PR #724 to combine both source attributes in one duration
calculation, rather than applying independent rounded duration changes.

Source: TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, generated
`balm_of_sssss.json`, Darkthread's skull-stat trait, `AttributeModule`,
`TinkerAttributes` and `ModifierEvents.onPotionStart`.

## Behavior

Each positive-level piece contributes `-0.1 - 0.1 * level` as MULTIPLY_BASE.
Level 1 is nominally a 20% reduction. Flat terms apply once per contributing
piece, not to empty or zero-level slots. Multiple pieces add their reductions
against the attribute's base of one; they do not multiply independently.

The combined base also includes [Magic Protection](magic-protection-duration.md)'s
strongest-equipped-level reduction before each Crystalstrike MULTIPLY_TOTAL contribution.
The resulting attribute is clamped to 0–100. Only then is the incoming duration
multiplied and truncated, with a minimum of one tick when changed. Infinite
(-1) duration is preserved. Float coefficient arithmetic before double widening
is retained: for example, a 100-tick effect becomes 79 ticks at one Balm level,
or 83 ticks with one Balm level plus one Crystalstrike level. Applying the
rounded Balm result and Crystalstrike separately would incorrectly give 82.

Known source milk-curable nonbeneficial effects use the existing allowlist,
including neutral Glowing and Bad Omen. Beneficial, instant, newer/custom effects
and unknown curative metadata are not guessed. Reaching a zero multiplier gives
a one-tick effect, not immunity. Existing active/hidden durations are not
retroactively shortened when equipment changes.

## Runtime coverage

- Resolve live material/fixed traits and integer upgrades on correct-slot stock
  worn armor, with the current worn tag and complete known definition.
- Percent-style AttributeModule equipment effects remain on broken pieces.
  Malformed levels/counts, opaque wire patches and unavailable source snapshots
  fail closed. Balm and Crystalstrike definitions fail independently.
- Reuse the existing `give_player_effect` hook and add the other native player
  entry point, `add_player_effect`. This covers their mob, drinking, food,
  tipped-arrow and splash/lingering-potion callers. The existing/hidden effect
  merge logic and amplifier handling remain native. Each entry point scales only
  the new instance once; neither currently delegates to the other.
- Read current equipment/resources at each application, with no saved counters
  or persistent derived duration bonus. Unequip and reload affect future effects.

## Deferred

Held equipment, custom armor classes, source custom effect/curative metadata,
duration modifiers other than Magic Protection, custom attribute commands, direct native effect-list
writes, and Darkthread skull acquisition proof are outside this patch. Remote
clients retain their native path. Actual save/reload, guest swaps, synchronization,
LAN and cross-mod hook ordering remain untested.

Brief static review and formatting only. Four regression tests were added for
source formulas, operation order, final duration rounding, infinity, zero clamp
and definition gates, but were not run. No tests, builds, planner, CI, version
checks or merges were run.
