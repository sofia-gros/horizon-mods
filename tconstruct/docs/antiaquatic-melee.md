# Antiaquatic acquisition and primary melee damage

This high-priority #379 slice builds on the local Smite implementation and
reuses its primary-target damage scope. Five pufferfish complete one level of
Antiaquatic, up to level V. Each started level consumes one upgrade slot.
Partial progress, matching modifier crystals, extraction and workstation/item
text use the same five-unit contract as the source recipe. Smite and
Antiaquatic each occupy their own slots when installed together.

The conditional melee bonus is +2 damage per effective level against a source
water-type target. This is a mob classification, not a test of whether a mob
is currently wet or underwater. The bounded vanilla mapping includes Cod,
Salmon, Pufferfish, Tropical Fish, Tadpole, Dolphin, Squid, Glow Squid, Guardian,
Elder Guardian, Axolotl and Turtle. Drowned, other undead, Frog and ordinary
land mobs do not gain water-type eligibility merely by entering water.
Unknown namespaces and newer/custom aquatic types are not inferred.

The shared runtime selects Smite for undead and Antiaquatic for the supported
water types, validates that modifier's current stock definition, and combines
material trait levels with partial recipe progress in source float order. The
existing common conditional multiplier and supported Jagged/Stonebound order
are preserved. The selected bonus enters before native cooldown and the actual
critical multiplier. There is no extra native Impaling enchantment or second
parallel damage hook.

## Bounds and deferred validation

The 15 held melee tool classes, intact/tool-tag requirements, primary main-hand
mob scope, actor/target context and saved-multiplier rebuild requirement are
those documented in `smite-melee.md`. Secondary targets, projectiles, offhand,
PvP, monster-held tools and arbitrary custom definitions remain outside this
slice. It does not finish the grouped Smite/Bane/Antiaquatic/Knockback/Pierce
issue row.

Brief static review covered the source pufferfish recipe, shared slot budget,
separate target predicates, and reuse of the single damage path. Regression
code was added for all twelve water-type IDs, namespace rejection, Drowned/Frog
exclusion, progress, shared slots, crystals and extraction. None was run.
No tests, build, Mixins transformation, CI inspection, version checks or merge
were performed. Gameplay, persistence, complete modifier ordering and LAN
remain unverified. Publication remains paused.

Sources:
- Tinkers' Construct f4e77d43,
  `recipes/tools/modifiers/upgrade/antiaquatic.json` and
  `tinkering/modifiers/antiaquatic.json`.
- Java MobType.WATER overrides in WaterAnimal, Guardian, Axolotl and Turtle;
  Tadpole extends AbstractFish. Reference source:
  https://github.com/mahtomedi/minecraft/tree/main/src/main/java/net/minecraft/world/entity
- Target API declarations for the source comparison:
  https://mappings.dev/1.20.1/net/minecraft/world/entity/animal/Turtle.html
  and the corresponding 1.20.1 Axolotl/WaterAnimal classes.
