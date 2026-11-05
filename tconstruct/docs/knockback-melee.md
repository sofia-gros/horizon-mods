# Ordinary Knockback: Sword and Cleaver

This bounded high-priority #379 slice adds the original Knockback upgrade to
Sword and Cleaver. A piston and a slime block in two occupied input slots, or
one matching modifier crystal, grants one level at a Station or Anvil. Levels
I–III consume one upgrade slot each. Extraction removes one purchased level,
returns the normal crystal, and leaves material trait levels intact. Item and
workstation text show the purchased level. Malformed or out-of-range saved
progress is not overwritten as an empty upgrade.

The source's main-hand attribute module adds +1 attack knockback per effective
modifier level. This implementation resolves material traits and stored levels
from the current world and sets a separate transient main-hand attribute. It
refreshes on equipment changes, player ticks, definition reload and immediately
before the primary mob attack calculation. Removing or invalidating the held
tool removes only this modifier; item enchantments are not rewritten.

The inspected host's primary-mob extra-knockback calculation reads native
enchantments and sprint but not the attack-knockback attribute. Its operation
wrapper therefore adds the supported TiC contribution, +0.5 per level, while
preserving the existing sprint flag and native contribution. The normal host
path performs the target push and sprint cancellation only after a landed hit.
A companion hurt wrapper applies the source's 0.6 horizontal attacker slowdown
only after a successful supported hit. The native knockback-resistance and
cancellation paths remain in charge of the target's response.

## Bounds

- Acquisition and runtime are enabled only for Sword and Cleaver. Dagger's
  Padded, Battlesign and other tool-specific knockback need ordered dispatch.
  Other tools, chestplate/worn attributes, projectiles, slings, offhand attacks,
  PvP, sweep/secondary targets and the host's Sulfur Cube are not implemented.
- Runtime requires intact material tools, live melee-tag eligibility, the
  exact stock Knockback modifier definition and no No Melee modifier. Broken
  tools, opaque wire patches and unsupported or missing definitions are not
  treated as verified source-compatible tools.
- Other active modifiers containing a source knockback module suppress this
  bounded path. External attribute multipliers and complete source modifier
  order are not reproduced by the primary-mob contribution wrapper.
- Remote clients use the authoritative attribute and receive no second local
  modifier. Actual guest equipment transitions, synchronization, gameplay and
  save/reopen remain unverified.
- The host's base knockback/push ordering is retained. This is not a claim of
  complete source vector or floating-point equivalence.

## Deferred validation

A brief static review covered source ingredients and slot cost, level limits,
extraction, transient modifier removal, operation-wrapper targets and successful
hit gating. Focused recipe and arithmetic regressions were added but not run.
No tests, build, Mixins transformation, CI inspection, version checks or merge
were performed. Publication remains paused.

Sources: pinned Tinkers' Construct f4e77d43,
`recipes/tools/modifiers/upgrade/knockback.json`,
`tinkering/modifiers/knockback.json`, and
`library/modifiers/modules/behavior/AttributeModule.java`.
