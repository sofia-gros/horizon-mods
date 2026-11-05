# Smite acquisition and primary melee damage

This bounded high-priority #379 slice admits the original incremental Smite
recipe on 15 existing held melee tool classes: Pickaxe, Sledge Hammer, Vein
Hammer, Hand Axe, Broad Axe, Mattock, Pickadze, Excavator, Kama, Scythe, Sword,
Cleaver, Dagger, Battlesign and War Pick. The live melee tag remains required.

Five glistering melon slices complete one level, up to level V. Partial progress
uses the original five-unit denominator and reserves the current upgrade slot;
starting the next level requires another slot. A matching crystal adds five
units. Extraction removes five units, returns the normal crystal, and retains
remaining progress and material traits. Unknown denominators, costs, level
limits and malformed saved progress do not silently use the stock math.

On primary attacks against a living mob marked undead by the host, the source
conditional module adds +2 damage per effective level. Material traits and
partial purchased progress are merged with the source float operation order.
The conditional attack multiplier is rebuilt during assembly, including the
existing Emerald contribution. Supported Jagged/Stonebound multipliers apply
to the new flat contribution in their existing order.

A primary-attack operation wrapper captures the actual target's undead status
and scopes the resulting bonus to that actor and target kind. A second wrapper
adds it to the attack-damage input before the host applies cooldown and the
actual critical multiplier, including native critical-hit modifications. The
native Smite enchantment is not used as a substitute: its coefficient is 2.5,
whereas this source modifier uses 2.0. Existing native enchantment processing
is preserved, and a positive source bonus marks the attack's magic effect.

## Bounds

- Primary main-hand attacks on mobs only. PvP, secondary/sweep targets,
  projectiles, offhand attacks, monster-held tools and additional tool classes
  are not connected. Reentrant attacks through the wrapped primary path get
  their own context; panic/unwind behavior is not established.
- Requires intact material tools, the live melee tag, no No Melee modifier,
  supported stock Smite definition and no opaque wire patch. Current host
  undead flags are used, not a claim that every newer/custom mob has an exact
  Java 1.20.1 counterpart.
- New acquisition/rebuild writes the conditional multiplier used here. Older
  trait-only tools without that field require rebuilding; this does not add a
  general saved-tool migration or rebuild all items after datapack changes.
- Other conditional melee modules and all combinations of source ordering,
  external attribute multipliers, effect particles and exact float/vector
  behavior remain separate integration work. Existing Raging and native attack
  behavior outside this new contribution are not rewritten.

## Deferred validation

Brief static review covered the original recipe, slot/progress accounting,
extraction, module ownership, target/actor scope and placement before cooldown
and critical evaluation. Regression code was added for partial progress,
trait merging, recipe limits, crystals and extraction. None was run.
No tests, build, Mixins transformation, CI inspection, version checks or merge
were performed. Gameplay, save/reopen and LAN remain unverified. Publication
is paused.

Sources, pinned Tinkers' Construct f4e77d43:
`recipes/tools/modifiers/upgrade/smite.json`, `tinkering/modifiers/smite.json`,
`ConditionalMeleeDamageModule.java`, `MeleeFormula.java`, `ToolAttackUtil.java`.
