# Rugged damage blocking

Related to high-priority #379. Stacked on the shared protection/status lifecycle;
Rugged does not add protection points or alter the protection cap.

## Added coverage

- Resolve the exact stock Rugged definition and live material traits plus saved
  integer modifier levels on supported stock worn armor and either-hand shields.
- An intact level-I piece blocks the live `tconstruct:rugged/terrain` tag: stock
  Hot Floor, Cactus, Sweet Berry Bush and Stalagmite, plus available tagged mod
  damage types. Level II additionally blocks `tconstruct:rugged/attacks`: stock
  Cramming, Sting and Thorns, plus optional Twilight Forest Thorns.
- Level requirements apply to each piece, not the equipped sum. Two level-I
  pieces do not unlock the level-II module. Broken pieces grant no immunity.
- Cache blocked type IDs on the player's transient status, refreshing on
  equipment changes, status replacement, native player tick and resource reload.
  Live tag replacement/removal is respected; unresolved tags disable only their
  own module. Unknown source-only damage IDs are not fabricated in the host.
- Cancel a matching hit at the native player's `HurtWorld::hurt_start` return,
  preserving prior native/mod cancellation and errors. This is before shield
  handling, armor wear, hurt cooldown, hit sounds, knockback and statistics.
  Set the native cancellation marker so callers discard the refused combat note.
  No durability charge, random roll, permanent invulnerability or NBT flag.

## Boundaries

This covers Rugged's two damage-block modules. Its separate `snow_boots` volatile
flag and powder-snow movement/freezing behavior are not implemented here. The
native bee attack path currently uses `mob_attack`, not `sting`, so that source
mismatch is not silently rewritten by this modifier. Cramming/Sting protection
applies when those actual damage types reach native health.

The 13 supported worn item IDs and three stock shield IDs are the same bounded
set as the shared defense bridge. Other held/custom tools and mob equipment are
not covered. Remote clients do not calculate local immunity. Final equipment
wear ordering, immediate same-tick changes outside the existing refresh hooks,
save/reload and full host/guest/LAN behavior remain unverified. Cancellation
ordering relative to every third-party Forge callback is not claimed identical.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`rugged.json`, `tags/damage_type/rugged/{terrain,attacks}.json`,
`BlockDamageSourceModule`, `DamageBlockModifierHook` and `ToolEvents.livingAttack`.

Pure regression cases were added for per-piece thresholds, broken/invalid data,
independent missing tags and definition overrides. They were not run. Brief
static review and `git diff --check` only. No tests, builds, planner runs, CI,
version checks or merge. Publication remains paused.
