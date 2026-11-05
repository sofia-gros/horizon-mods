# Bane of Sssss acquisition and primary melee effects

This #379 slice extends the local Smite/Antiaquatic implementation. Fifteen
fermented spider eyes complete one Bane of Sssss level, up to V. Partial
progress reserves the current upgrade slot; a new level requires another slot.
A matching crystal grants 15 units, and extraction removes 15 units while
preserving remaining progress and material traits. Bane, Smite and Antiaquatic
are counted separately in the shared upgrade-slot budget and UI.

Bane adds +2 damage per effective level against vanilla arthropods (Spider,
Cave Spider, Silverfish, Endermite and Bee), or members of the current world's
`forge:creepers` entity tag. Tag removal/replacement is respected; a Creeper
is not unconditionally hardcoded eligible. The predicate is the union of both
conditions, so an arthropod does not require creeper-tag membership. Custom
creeper-tag members can qualify, but custom arthropods are not inferred.

The shared primary damage path now evaluates each supported target modifier
independently. If tag changes make predicates overlap, their applicable levels
contribute to the flat bonus rather than one target branch hiding another.
The common conditional multiplier and supported wear-percentage treatment are
reused before native cooldown and critical scaling.

A successful primary hit with source cooldown strictly above 0.9 also applies
Slowness IV (native amplifier 3). Duration follows the source float expression
`20 + random_float * effective_level * 10`, truncated to integer ticks. Partial
levels affect duration as well as damage. The effect is checked at the primary
weapon-hit boundary before native weapon wear, using the matching target's
landed attack record. Weak, missed, mismatched and dead-target hits do not
apply it. Existing native effect merging handles an already active Slowness.

## Bounds and deferred validation

The same 15 held melee classes and intact/tagged material-tool checks as
`smite-melee.md` apply. Only primary main-hand mob hits are connected. Other
classes, offhand, PvP, secondary targets, monster-held tools and projectiles
remain outside this slice. Custom source definitions are rejected. The saved
conditional-multiplier rebuild requirement and source-order limitations remain.
Overlapping custom predicates use the supported aggregate flat contribution;
bit-exact ordering across arbitrary modifier modules is not claimed.

Brief static review covered the original 15-unit recipe, level/slot accounting,
world-overlay entity tags, source fully-charged condition, amplifier numbering,
random duration and successful-hit placement. Focused regression code was
added for recipe limits/extraction, fractional levels, arthropod classification
and duration arithmetic, but was not run. No tests, build, Mixins transformation,
CI inspection, version checks or merge were performed. Gameplay, persistence,
complete reentrancy/unwind behavior and LAN remain unverified. Publication is
paused.

Sources: Tinkers' Construct f4e77d43,
`recipes/tools/modifiers/upgrade/bane_of_sssss.json`,
`tinkering/modifiers/bane_of_sssss.json`, `forge/tags/entity_types/creepers.json`,
`MobEffectModule.java`, `RandomLevelingValue.java` and `ToolAttackContext.java`.
