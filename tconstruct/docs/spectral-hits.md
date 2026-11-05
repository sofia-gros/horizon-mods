# Spectral: bounded melee, arrow status, and block light

Related to #379. Source reference: Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, generated `spectral.json`,
`MobEffectModule.WeaponCommon`, `ProjectileHitModifierHook`, and
`ProjectilePlaceGlowModule`.

## Implemented

- A successful, fully charged (`cooldown > 0.9`) mainhand attack on a living
  mob applies Glowing I for `200 * level` ticks. Live material traits/upgrades,
  melee tag, intact-tool and exact source-module checks gate eligibility.
  The existing Kama/Scythe circle attacks also use this attack path; the
  Sword/Cleaver sweep shortcut does not. Ambiguous nested scopes fail closed.
- Native TiC longbow/crossbow arrow launches snapshot Spectral's duration onto
  the arrow's retained firing-stack copy. This marker does not change the
  held weapon or arrow pickup item. Later tool switching or material changes
  do not change the in-flight duration. A changed Spectral definition disables
  impact processing.
- The source five-durability launch cost runs only for a primary arrow,
  after Glowing's higher-priority cost, through existing Overslime/durability
  handlers with the `tconstruct:spectral` cause. Secondary arrows inherit the
  effect without paying again. Launcher identity checks permit only wear
  changes between the firing snapshot and committed stack.
- Successful ordinary arrow damage to a surviving mob applies Glowing I for
  the captured duration. Native effect updates/ticking/replication are reused.
- Spectral arrows hitting blocks share the existing supported-face light
  placement and actor spawn-protection checks. Successful placement removes
  the tracked arrow. Spectral alone never places light at an entity's feet;
  a separate Glowing modifier still can. Failed placement leaves the arrow.
- Preserve native firing-stack snapshots when the existing Glowing launch
  handler runs, rather than replacing them with a newly worn held stack.

## Remaining boundaries

Armor/shield counters, PvP/player targets, monster-held tools, offhand melee,
Sword/Cleaver sweep callbacks, rockets, thrown projectiles, fishing hooks,
custom definitions/effective levels, and acquisition changes are not covered.

The arrow status hook is intentionally narrower than source's *unblocked
impact* callback: damage-refused/invulnerable/dodged targets are not given the
status. Source applies it before native projectile damage; this implementation
applies it afterward. Melee status likewise runs at successful damage return,
before later native hit callbacks. Full event/modifier-order parity is deferred.

Actual bow/crossbow shooting, multishot, combined Glowing/Spectral wear order,
block placement/removal, save/reload of the firing-stack marker, effect visuals,
and LAN behavior have not been exercised. Existing source-module and native
serialization limits remain; marker-less older arrows gain no new effect.

Brief static review and formatting only. Two pure regression tests were added
but not run. No tests, builds, planner runs, CI inspection, version checks, or
merges were performed.
