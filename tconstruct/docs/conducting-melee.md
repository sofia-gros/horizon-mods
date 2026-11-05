# Blazing Bone Conducting primary melee

This #379 slice connects Blazing Bone's default Conducting trait to the existing
15 intact main-hand melee tools. The material's ammo/armor/ribcage Conductive
status trait is different and is not used here. No recipe or purchased modifier
is added. Current material traits, live melee tags and the static modifier's
absence of a dynamic override are checked on each primary attack.

The source's multiplicative bonus is:

- Remaining fire ticks at or below zero: no bonus.
- Positive fire: `min(fire_ticks / 200, 1) * level * 0.2`.
- Active Fire Resistance halves the fire scale, rather than removing the bonus.
- Damage is multiplied by `1 + bonus`.

The cap is 200 ticks (10 seconds), not the source comment's outdated 15 seconds.
Remaining fire duration is authoritative even when resistance hides burning
presentation. The active player's native fire timer and potion list are sampled
once into the existing scoped primary-melee context; no tool stat is mutated
or saved. Remote prediction, broken tools, spin, offhand and unsupported item
shapes remain excluded.

The factor multiplies the grouped base and supported flat contributions before
native cooldown and actual critical-hit scaling. The existing separate Raging
return-path contribution also receives it once, before Raging's wear percentage
adjustment. The primary scope prevents it from becoming a permanent attribute
bonus or leaking into unrelated damage calls. At zero fire duration, prior
behavior is retained.

## Bounds and deferred validation

Only primary melee is connected. Source projectile-damage scaling, monster-held
melee and Conducting's dynamic tooltip are not part of this slice. Conductive
status application and its fire-damage amplification remain separate work.
This is integer material-level support; arbitrary effective-level hooks are not
interpreted. Dynamic replacement of the static Conducting definition fails
closed rather than inheriting stock behavior.

Existing grouped wear/flat modifier ordering and floating-point rounding limits
remain; this does not claim bit-exact evaluation of arbitrary modifier orders.
The pre-existing Raging return path's critical handling is unchanged. No new
PvP, secondary-target, unarmed armor, material acquisition or saved-data schema
coverage is claimed.

Brief static review covered the 200-tick cap, resistance halving, source float
operation order, scoped context lifecycle and all context tuple consumers.
Pure formula/composition regressions were added but not run. No tests, builds,
Mixins transforms, CI inspection, version checks or merge were performed.
Gameplay, combined effects, save/reload and LAN remain unverified.
Publication is paused.

Sources: Tinkers' Construct f4e77d43, `ConductingModifier.java`,
`TinkerModifiers.java` and `tinkering/materials/traits/blazing_bone.json`.
