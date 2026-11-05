# Insatiable: bounded ordinary-arrow accumulation and damage

Related to #379. This change is stacked on #772's melee effect implementation.
Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`InsatiableModifier.modifyStat/onProjectileHitEntity`, `TinkerEffect`, and
`TinkerModifiers.insatiableEffect`.

## Implemented

- Each successful ordinary TiC longbow/crossbow arrow hit on a mob grants its
  owning player one ranged effect level, capped at five, for 200 ticks. At cap,
  another qualifying hit refreshes the duration. This effect is independent
  of the melee effect and is stored on the player, not their weapon.
- At arrow launch, active ranged stacks add
  `effectLevel * modifierLevel / 4 * projectileStatMultiplier` to the TiC
  projectile-damage stat. Source float arithmetic and the 0..1024 stat clamp
  precede conversion back to the native stat-minus-two arrow delta; native
  enchantment damage remains separate. Current source traits/upgrades and
  ranged tags gate the modifier; the existing conditional multiplier field
  is reused. No Insatiable-specific durability cost is introduced.
- A launch marker in the arrow's retained firing-stack copy records that the
  shot had Insatiable. Impact credit does not depend on which tool is held
  later or on material changes while the arrow is in flight. A missing marker
  gives no buff. Marking does not mutate the held weapon or pickup item.
- Damage is captured before the projectile hits, so a hit's newly earned stack
  affects later launches, not arrows already in flight. Multishot arrows share
  the current buff and each successful hit can add a stack, matching the
  absence of a primary-only guard in the source impact hook.
- Register the source-colored beneficial ranged effect, with particles off.
  Reuse native effect updates/ticking/save/network paths and the #772 milk
  wrapper, which now preserves both Insatiable variants and Momentum.
- Ownership checks reject departed/mismatched guests, mismatched player UUIDs,
  mob owners, and dead shooters; native guest-context swapping supplies the
  authoritative player. The Glowing launch handler preserves the native
  pre-wear firing snapshot, matching the same adjustment in Spectral #777.

## Remaining boundaries

This grants stacks after *successful damage to mobs*. Source grants them on
all unblocked entity impacts, so refused/invulnerable/dodged hits and nonliving
entities remain outside this bounded connection. PvP/player targets, rockets,
slings, thrown tools and monster shooters are unimplemented. Old arrows without
markers and unsupported source definitions do not acquire the effect.

Insatiable is added after the existing Raging/Ambush conditional calculation.
Full equal-priority material/upgrade ordering and bit-identical mixed-trait
rounding remain deferred. Unknown/custom effective levels and migration of
old tools missing the stored conditional multiplier are not implemented.

Actual bow/crossbow/multishot behavior, hidden-effect persistence, firing-marker
save/reload, LAN ownership and effect visuals have not been exercised. Source
icon artwork is not added. This is not completion of the broad issue.

Brief static source/host review and formatting only. Two ranged pure tests were
added, not run. No tests, builds, planner runs, CI inspection, version checks,
or merges were performed.
