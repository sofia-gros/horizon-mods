# Venombone Antitoxin melee and draw bonuses

This #379 slice connects Venombone's default Antitoxin material trait. Its
armor/ammo-specific Venom trait remains distinct. There is no new recipe or
purchased upgrade. The actual actor must have a currently active Poison
effect, and the highest current Antitoxin definition must match the source.
Malformed or invented purchased Antitoxin entries are rejected.

The source effect variable is one-based: Poison I is 1, not amplifier 0.
For trait level L and the appropriate source stat multiplier M, the bonuses
follow the original float expression order:

- Primary melee damage: `(poison_level + 0.5) * L * M`.
- Launcher draw speed: `(poison_level + 0.5) * 0.1 * L * M`.

Melee uses the same 15 known intact main-hand tools and live melee tag as the
preceding target-damage slices. The flat bonus joins their scoped contribution
before the native cooldown and actual critical multiplier. Supported Jagged
and Stonebound percentage effects are applied afterward. There is no new
persistent damage bonus and no benefit while unpoisoned.

Longbow release uses the current Poison state to derive charge and projectile
power from the existing draw-speed pipeline. Crossbow loading and its loading
sounds use the current effective draw duration each tick. Its local item-owner
render data receives the same duration. Charge is recalculated when Poison
changes or expires; saved tool stats are not modified. Release validation
accounts for the native `using` state being cleared before the bow callback.
The authoritative acting-player swap supplies that player's Poison state.

The existing stored conditional multipliers are reused: the melee multiplier
includes applicable tool multipliers and Emerald, while the draw multiplier
includes the tool definition and Vintage, not base-only Quick Charge. Existing
Overwield draw addition remains in the calculation. Tools lacking the required
saved multiplier must be rebuilt through the existing Station path.

## Static hook correction included in this stack

Commit 72e82086 corrects earlier Smite/Pierce/Enderference declarations: the
Mixins API rejects extra context arguments on operation wrappers. The shared
melee context now uses before/after-call hooks, the damage result uses an
expression modifier, and pre-hit effects receive the actual cooldown from
`melee_damage`'s returned tuple. The mob armor wrapper also matches its target's
mutable receiver. This was a source/API correction, not a successful transform
or compilation claim.

## Bounds and deferred validation

Melee remains primary mob-only: no new PvP, offhand melee, secondary targets,
monster-held or projectile damage behavior. Draw support is limited to known
intact Longbow/Crossbow items in the live launcher tag. Remote-client prediction,
third-person player-model charge and gamepad feedback are not newly connected.
Poison immunity/removal itself is unchanged. Arbitrary modifier ordering and
bit-exact floating-point addition across every trait combination are not
claimed; the existing supported flat/wear and Overwield paths are retained.

Brief review covered the source formulas, one-based Poison level, current actor
state, release timing, conditional multipliers, snapshot gates and non-mutating
draw calculation. Regression code was added for formulas and draw arithmetic
but not run. No tests, build, Mixins transformation, CI inspection, version
checks or merge were performed. Gameplay, combined modifier behavior, saved
state and LAN remain unverified. Publication is paused.

Sources: Tinkers' Construct f4e77d43,
`tinkering/modifiers/antitoxin.json`, `tinkering/materials/traits/venombone.json`,
`EntityEffectLevelVariable.java` and `ConditionalStatModule.java`.
