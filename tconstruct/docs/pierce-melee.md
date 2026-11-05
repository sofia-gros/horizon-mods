# Armor Pierce acquisition and primary melee effect

This #379 slice extends the local Smite/Antiaquatic/Bane work. Ten Punji
complete one purchased Armor Pierce level, up to III. Each started level
reserves an upgrade slot; completing its partial progress uses the same slot.
Crystals add ten units, extraction removes ten units, and both Station and
item UI use that denominator. The existing 15 known held melee classes are
accepted with the current world's melee tag. Source launcher recipes remain
closed here because projectile application is not yet connected.

Purchased progress is merged with the material trait's integer level before
the source float partial-level correction. The existing +0.5 attack-damage
stat per level now includes purchased and partial levels as well as Bone's
trait, and continues through the ordinary source stat multiplier pipeline.
Rebuilding uses the baseline stats, so it does not accumulate the bonus.

A primary main-hand attack with cooldown strictly above 0.9 applies the
registered harmful `tconstruct:pierce` effect for 80 ticks before damage.
Its one-based effect level is Java-style `Math.round(effective_level)`, so
half a level grants Pierce I, but less than half does not. Application before
damage is intentional: source `apply_before_melee` can leave the effect even
when the damage is subsequently rejected. The target must still be alive,
the actual selected tool must be intact and recognized, and the current
highest modifier definition must match the stock modules.

During the target mob's damage calculation, the active effect subtracts one
armor point per effect level from the total used by the native armor formula.
Innate, worn and closed-Shulker armor are therefore reduced together. Toughness,
armor-bypassing sources, Breach processing and native equipment wear retain
their normal paths. Neither shared MobKind data nor equipment is mutated.
The effect continues to affect later damage even after switching weapons,
and stops affecting armor when its timer expires or the effect is removed.
The native effect list handles its timer, replacement and saved representation.

## Bounds and deferred validation

Only primary main-hand mob-hit application is connected, with the same local
or authoritative acting-player context as the preceding melee slices. No new
projectile, launcher, secondary-target, offhand, monster-held, PvP, unarmed
armor or Piercing Guard application is claimed. Player armor is not a consumer
of this effect in this slice. Custom modifier definitions fail closed for new
acquisition and runtime application; the pre-existing saved-stat rebuild
limitations remain. Native merging lacks the source's hidden weaker-effect
queue, so restoring a weaker effect after a stronger one expires remains
outside this implementation. Effect levels above 256 are rejected.

Brief static review covered recipe bounds and extraction, merged partial
levels, effect registration, charge timing, the pre-damage boundary, mob
armor composition and native effect expiry. Regression code was added for
acquisition, slots, crystals, extraction, stat rebuild and effect rounding,
but was not run. No tests, build, Mixins transformation, CI inspection,
version checks or merge were performed. Gameplay, save/reload and LAN remain
unverified. Publication is paused.

Sources: Tinkers' Construct f4e77d43,
`recipes/tools/modifiers/upgrade/pierce.json`, `tinkering/modifiers/pierce.json`,
`MobEffectModule.java`, `TinkerEffects.java` and `IncrementalModifierEntry.java`.
The deprecated `PiercingModifier.java` is not the specification for this
current Armor Pierce modifier.
