# Scorched Stone Scorching primary melee

This #379 slice connects Scorched Stone's default Scorching trait to the
existing 15 known intact main-hand melee tools. The armor-specific Scorch
Protection and Blaze ribcage's unarmed Scorching effect remain separate.
No new recipe or purchased modifier is added.

Against a mob whose native `Entity.isOnFire` predicate is true, Scorching
contributes +2 damage per material trait level, multiplied by the source
conditional attack-damage multiplier. Positive fire ticks on a fire-immune
mob do not qualify. The predicate is sampled during primary damage calculation;
this does not automatically grant the bonus for ignition later in the same hit.

The contribution joins supported flat target/actor modifiers, then the existing
Jagged/Stonebound percentage treatment, native cooldown and actual critical
multiplier. Burning targets can also receive applicable Smite, Bane or
Antiaquatic damage; the predicates do not replace each other. Current material
traits, live melee tags and the exact highest Scorching definition are checked.
Malformed or invented purchased Scorching entries are rejected.

## Bounds and deferred validation

No new PvP, offhand, secondary-target, monster-held, projectile, unarmed armor
or Scorch Protection behavior is claimed. The preceding melee slices' saved
multiplier/rebuild and arbitrary modifier-order limitations remain. Damage is
read from current target state, not cached into the tool.

Brief static review covered the original +2 coefficient, the native burning
predicate, Scorched Stone's armor/default distinction, current definition gate
and placement before cooldown/critical scaling. Regression code was added for
trait/predicate eligibility and changed-definition rejection but not run.
No tests, build, Mixins transformation, CI inspection, version checks or merge
were performed. Gameplay, combined modifiers, save/reload and LAN remain
unverified. Publication is paused.

Sources: Tinkers' Construct f4e77d43,
`tinkering/modifiers/scorching.json`,
`tinkering/materials/traits/scorched_stone.json`,
`tinkering/materials/traits/blaze.json` and `ConditionalMeleeDamageModule.java`.
