# Weeping Vine Flamestance combat bonuses

This #379 slice connects the held combat effects of Weeping Vine's Flamestance
trait. Its laces-specific Soul Speed trait is separate. No new acquisition
recipe or armor-protection behavior is added.

The actual actor's block position selects the biome using the world's seeded
eight-corner biome zoom and current biome table. The temperature is sampled at
that original position, including the frozen modifier and cooling above Y=80
from the source `getTemperature(BlockPos)` convention. The host temperature
helper expresses this as sea level 63 plus 17. A missing selected biome chunk,
invalid coordinates or nonfinite temperature suppresses the bonus instead of
inventing a warm default. World pack biome overrides are used.

For temperature T and the matching source stat multiplier M:

- Primary melee damage changes by `(T - 0.75) * M`.
- Launcher projectile damage changes by `(T - 0.75) * 0.6 * M`.

Flamestance is level-independent. Cold biome penalties remain negative; they
are not silently discarded. Primary melee uses the existing intact main-hand
15-tool scope, joins the flat contribution before supported wear-percentage
modifiers, and then follows native cooldown and critical scaling. Magic-hit
feedback is not added merely for a negative contribution.

Longbow and Crossbow arrow shots sample the firing actor's current biome.
The projectile stat applies Flamestance with Raging and Ambush in saved trait
order, then clamps once before conversion to the native arrow-damage delta.
This preserves a cold penalty even when other bonuses follow it. No saved
tool stats are changed, and arrows already fired retain their launch damage
when the shooter moves between biomes. Existing projectile-context handling
covers the supported arrow payloads, including Crossbow's nonrocket shots.

## Bounds and deferred validation

Only stock highest-priority Flamestance definitions and positive material
traits are accepted. Opaque purchased entries, broken/unknown tools, missing
live tags and remote prediction are excluded. Existing saved conditional
multipliers are required, so older tools may need a Station rebuild.
Armor protection, player/PvP and secondary melee, monster-held tools, rockets,
throwing/sling weapons and custom modifier formulas remain outside this slice.
Source-exact temperature noise and all cross-modifier floating-point behavior
are not newly verified; this uses the existing native biome sampler and the
bounded combat paths documented in the preceding slices.

Brief static review covered source sign and level independence, actor feet
position, zoomed/current biome lookup, altitude/frozen temperature adjustment,
conditional multipliers, arrow launch timing and final clamping. Regression
code was added for signed formulas and projectile ordering/clamping but not
run. No tests, build, Mixins transformation, CI inspection, version checks or
merge were performed. Gameplay, save/reload and LAN remain unverified.
Publication is paused.

Sources: Tinkers' Construct f4e77d43,
`tinkering/modifiers/flamestance.json`,
`tinkering/materials/traits/weeping_vine.json`, `EntityVariable.java`,
`ConditionalStatModule.java` and `ConditionalMeleeDamageModule.java`.
