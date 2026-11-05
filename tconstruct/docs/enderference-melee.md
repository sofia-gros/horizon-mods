# Enderference primary melee and mob teleport suppression

This #379 slice connects Chorus's default Enderference material trait to the
existing 15 known held melee-tool classes. It extends the local primary melee
work without adding a new acquisition recipe. Material definitions and the
current world's melee tag remain authoritative; Chorus's armor-specific
Enderclearance trait is not substituted with Enderference.

An intact, recognized selected main-hand tool with the positive trait applies
Enderference I for 100 ticks on a fully charged primary mob attack (cooldown
strictly above 0.9). The source duration and amplifier are flat, independent
of the trait level. Application happens before damage, as specified by the
source module, so a later rejected hit can still leave the effect. Altered
highest-priority modifier definitions and opaque saved purchased entries
fail closed. Client prediction does not apply the effect.

The harmful effect is registered with its source color and existing translated
name. Active Enderference cancels the native teleport boundary for:

- Endermen's random, towards-target, daylight and damage-triggered teleports,
  which all reach `teleport_to`.
- Shulker teleports, including the move required before duplication.
- Foxes' chorus-fruit random teleport attempts.

The guards return failure before the actual position change or teleport
sound/particle events. They do not change effect duration, equipment, shared
mob definitions or the requested destination. Native effect ticking and
removal control when teleporting becomes available again. Native callers
may still draw candidate locations or perform preceding actions (for example,
a fox may dismount); source-exact random-consumption ordering is not claimed.

## Bounds and deferred validation

Projectile application and the special arrow-damage override for affected
Endermen are not implemented. Affected Endermen still retain the host's
projectile-damage immunity, although their attempted teleport is suppressed.
Armor counters, PvP/player teleportation, offhand, secondary hits, monster-held
tools, Ender Slime behavior, commands, cross-dimension movement, custom
teleport implementations, potions and fluid contact are outside this slice.
Other contributors' fluid work is unchanged. Native weaker-effect restoration
limitations from the preceding Pierce slice also remain.

Brief static review covered the source before-hit/fully-charged module,
Chorus's per-stat trait distinction, exact snapshot gate, shared native effect
timer, and each teleport boundary's failure return. Regression code was added
for expiry and definition rejection but was not run. No tests, build, Mixins
transformation, CI inspection, version checks or merge were performed.
Gameplay, save/reload, effect synchronization and LAN remain unverified.
Publication is paused.

Sources: Tinkers' Construct f4e77d43,
`tinkering/modifiers/enderference.json`, `tinkering/materials/traits/chorus.json`,
`MobEffectModule.java`, `ModifierEvents.onTeleport` and `TinkerEffects.java`.
