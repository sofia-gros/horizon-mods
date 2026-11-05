# Source references

The slime plant support rules in
`mixins/tconstruct/source-slime-plant-support.rs` were implemented with
reference to Tinkers' Construct's `SlimeTallGrassBlock.java` at
[f4e77d43f39ba1047bf93ae816b7e1beaab514c2](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/world/block/SlimeTallGrassBlock.java).
Copyright (c) 2022 SlimeKnights. The original is MIT licensed; see
`tconstruct/third_party/tconstruct/LICENSE`.

The slime grass bonemeal algorithm in `mixins/tconstruct/source-slime-grass-meal-algorithm.rs`
is adapted from Tinkers' Construct [SlimeGrassBlock.java at f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/world/block/SlimeGrassBlock.java).
Copyright (c) 2022 SlimeKnights; MIT license, see `tconstruct/third_party/tconstruct/LICENSE`.
The slimy-grass tag files are from the same revision.

The slime seed conversion rules refer to Tinkers' Construct
[SlimeGrassSeedItem.java at f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/world/item/SlimeGrassSeedItem.java),
and the five slime grass seed textures are copied from that revision.
Copyright (c) 2022 SlimeKnights; MIT license, see `tconstruct/third_party/tconstruct/LICENSE`.
The sound and registration checks also consulted
[TinkerWorld.java at f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/world/TinkerWorld.java), under the same MIT license.

Mighty Smelting `sections/tools.json`, `en_us/tools/tips.json`, and
`en_us/tools/designing.json` are from Tinkers' Construct
[f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/resources/assets/tconstruct/book/mighty_smelting).
Copyright (c) 2022 SlimeKnights; MIT license, see `tconstruct/third_party/tconstruct/LICENSE`.

The modifier resource archive and modifier tag files are generated data from
Tinkers' Construct [f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering).
The registry lifecycle was checked against [ModifierManager.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/ModifierManager.java).
Copyright (c) 2022 SlimeKnights; MIT license, see `tconstruct/third_party/tconstruct/LICENSE`.
The bounded modifier classification order also references MIT-licensed
[JsonRedirect.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/json/JsonRedirect.java)
at the same revision.
Pig Iron piston behavior was checked against MIT-licensed
[TinkerMaterials.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/shared/TinkerMaterials.java),
[OrientableBlock.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/shared/block/OrientableBlock.java), and
[TinkerModule.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/common/TinkerModule.java).
Canonical modifier tag lookup was checked against MIT-licensed
[ModifierManager.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/ModifierManager.java) and
[ModifierId.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/ModifierId.java).
The bounded module-free modifier validation references MIT-licensed
[ComposableModifier.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/impl/ComposableModifier.java),
[ModifierLevelDisplay.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/util/ModifierLevelDisplay.java), and
[ModifierTooltipsField.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/util/ModifierTooltipsField.java).
The condition-field handling also references MIT-licensed Mantle
[RecordLoadable6.java](https://github.com/SlimeKnights/Mantle/blob/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/main/java/slimeknights/mantle/data/loadable/record/RecordLoadable6.java).
Incremental modifier/trait stat composition was checked against MIT-licensed
[ModifierEntry.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/ModifierEntry.java),
[StatBoostModule.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/modules/build/StatBoostModule.java), and
[FloatToolStat.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/stat/FloatToolStat.java).

The bounded advanced part material-ID line references MIT-licensed
[ToolPartItem.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/part/ToolPartItem.java),
[MaterialVariantId.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/materials/definition/MaterialVariantId.java), and
[TinkerToolParts.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tools/TinkerToolParts.java).

The chest-slot Haste attribute check uses the pinned MIT modifier data and
[AttributeModule.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/modules/behavior/AttributeModule.java).

Blasting's recipe and mining formula were checked against Tinkers' Construct
[f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2), including
`data/tconstruct/tinkering/modifiers/blasting.json`, its incremental recipe,
and `ModifierFormula.java` and `PostFixFormula.java` for effective levels.
The order of armor Haste and tool speed hooks was checked against
[`ToolEvents.java`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tools/logic/ToolEvents.java). Copyright (c) 2022 SlimeKnights; MIT license,
see `tconstruct/third_party/tconstruct/LICENSE`.

The potion, water and airborne mining multiplier order was checked against
[BreakSpeedContext.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/hook/mining/BreakSpeedContext.java)
at Tinkers' Construct f4e77d43. Copyright (c) 2022 SlimeKnights; MIT license,
see `tconstruct/third_party/tconstruct/LICENSE`.

Hydraulic's mining data and incremental recipes were checked against
[Tinkers' Construct f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2).
Its eye-water and rain predicates were checked against MIT-licensed Mantle
[LivingEntityPredicate.java](https://github.com/SlimeKnights/Mantle/blob/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/main/java/slimeknights/mantle/data/predicate/entity/LivingEntityPredicate.java)
and [HasEnchantmentEntityPredicate.java](https://github.com/SlimeKnights/Mantle/blob/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/main/java/slimeknights/mantle/data/predicate/entity/HasEnchantmentEntityPredicate.java).
Copyright (c) SlimeKnights; MIT licenses, see the respective files in
`tconstruct/third_party/`.

Copper Gauge's capability, support and display behavior was checked against
MIT-licensed [Mantle GaugeBlock.java](https://github.com/SlimeKnights/Mantle/blob/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/main/java/slimeknights/mantle/block/GaugeBlock.java)
and the pinned Tinkers' Construct CastingTankBlockEntity and FluidCannonBlockEntity.
Copyright (c) SlimeKnights; see the corresponding licenses in this directory.

The Copper Gauge potion-fluid display follows MIT-licensed
[Tinkers' Construct PotionFluidType.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/fluids/PotionFluidType.java).
Copyright (c) 2022 SlimeKnights; see `tconstruct/third_party/tconstruct/LICENSE`.

The Clear Tinted Glass base texture is copied
from Tinkers' Construct at [f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/resources/assets/tconstruct/textures/block).
Copyright (c) 2022 SlimeKnights; MIT license, see
`tconstruct/third_party/tconstruct/LICENSE`. The content package carries this
license at `assets/tconstruct/licenses/tconstruct.txt`.

The Piggybackpack item texture is from Tinkers' Construct
[f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/resources/assets/tconstruct/textures/item/gadgets).
Copyright (c) 2022 SlimeKnights; MIT license, see `tconstruct/third_party/tconstruct/LICENSE`.

The milk still texture and animation are unmodified copies of
[MinecraftForge at 243490d78001917c7e6ee6e94b45ef98776aa36f](https://github.com/MinecraftForge/MinecraftForge/tree/243490d78001917c7e6ee6e94b45ef98776aa36f/src/main/resources/assets/forge/textures/block)
(`milk_still.png` and `milk_still.png.mcmeta`). See the Forge LGPL-2.1 license
in `tconstruct/third_party/forge/LICENSE`.

The Cheese Ingot texture and item model are copied from Tinkers' Construct
[f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/resources/assets/tconstruct).
Copyright (c) 2022 SlimeKnights; MIT license, see `tconstruct/third_party/tconstruct/LICENSE`.

The Cheese Block model, blockstate and textures are copied from Tinkers' Construct
[f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/resources/assets/tconstruct).
Copyright (c) 2022 SlimeKnights; MIT license, see `tconstruct/third_party/tconstruct/LICENSE`.

The Venombone item texture and model are copied from Tinkers' Construct
[f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/resources/assets/tconstruct).
The model's vanilla parent is explicitly namespaced for Horizon.
Copyright (c) 2022 SlimeKnights; MIT license, see `tconstruct/third_party/tconstruct/LICENSE`.

The Mighty Smelting Seared Soul Glass identity and source fixture are from
[Tinkers’ Construct f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/resources/assets/tconstruct/book/mighty_smelting/en_us/smeltery/soul_glass.json).
The displayed text is adapted to explain existing acquisition and port limits.
Copyright (c) SlimeKnights; MIT license, see `tconstruct/third_party/tconstruct/LICENSE`.

The Fantastic Foundry acquisition and recipe-unlock advancements are adapted
from Tinkers' Construct at
[f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/advancements):
`foundry/fantastic_foundry.json` and `recipes/misc/common/fantastic_foundry.json`.
Copyright (c) 2022 SlimeKnights; MIT license, see
`tconstruct/third_party/tconstruct/LICENSE`. The native port adapts the display
icon key, recipe ID and recipe-unlocked predicate key.

The Tinkers' Gadgetry item texture, cover/page images, book index, introduction,
language labels, recipe and acquisition advancement are copied or adapted from
[Tinkers' Construct f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2).
Source paths are `src/main/resources/assets/tconstruct/book/tinkers_gadgetry`,
`book/images/covers/gadgetry_*.png`, `textures/item/tinkers_gadgetry.png`, and
`src/generated/resources/data/tconstruct/{recipes/common,advancements/world}/tinkers_gadgetry.json`.
Copyright (c) 2022 SlimeKnights; MIT license, see
`tconstruct/third_party/tconstruct/LICENSE`. Native icon/recipe formats and the
partial-edition notice are port adaptations.
