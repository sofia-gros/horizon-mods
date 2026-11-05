# Scorch Protection: identified causing attackers (#379)

This extends Scorch Protection's existing enchantment slice with its conditional
1.25 protection points per intact modifier level. It checks the causing mob's
actual native entity-type `fire_immune` flag, not whether the damage is fire,
the target is burning, or a nearby mob happens to resist fire.

## Covered native entry points

- `mob_hits_player` with a recorded projectile owner and an active native
  projectile/rocket source, including the existing small-fireball player hit.
- `mob_hits_player_caused` with an explicit causing-mob ID.
- `handle_mob_events`' explicit `CubeHurtPlayer` call, including Magma Cubes.
- `wither_skull_hit`'s explicit causing owner.

Explicit absence does not fall back to a stale owner. Missing/removed owners or
ordinary position-only melee hits receive no conditional points. The native
nearest-mob guess is deliberately never used. The wrappers bind their context
to the current damage type and restore prior per-actor context after the
synchronous call, including canceled hits. The existing Melee Protection
wrapper delegates to this scope so two transformations do not target its same
call expression. Cube/known/skull hooks do not expand Melee Protection's scope.

## Protection calculation

Canonical source-definition, material, saved-level, stock equipment and live-tag
guards remain in effect. Intact eligible worn/held pieces contribute 1.25 EPF
per level only when the causing-attacker condition is proven. Broken pieces
contribute none of this conditional protection. Their existing virtual Fire
Protection enchantment behavior remains separate: broken worn armor can still
add its native fire-only EPF, while broken held items cannot.

Conditional points join native and other modifier protection once, at the
existing post-armor/Resistance stage and its native bypass/cap handling. The
prior Fire Protection duration/contact behavior is unchanged. Player damage
remains server-authoritative; this adds no persisted fields or native changes.

## Remaining boundaries

Ordinary mob melee without an ID, inferred positions, generic `/damage`, other
unwrapped entry points, arbitrary custom tools, and unloaded projectile owners
are not claimed. This is not complete Scorch Protection parity. Exact runtime
ordering, canceled/nested calls, save/reload and LAN still need final validation.

Stacked on `feat/boundless-acquisition-379` (`afc0f85b`), retaining native cap
`5e47547e2` and signed-protection dependencies. No fluid-propulsion draft is
included. Specification: TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`
(MIT), canonical Scorch Protection definition and equipment protection hooks;
Mantle's causing-entity fire-immunity predicate. No external implementation copied.

Brief static review and whitespace checks only. Pure identity/arithmetic cases
were added but not run. No tests, builds, planner transformations, CI, version
checks, merges or publication were performed.
