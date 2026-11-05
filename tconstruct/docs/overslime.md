# Overslime and Overforced (in progress)

> **Provisional build:** Some effects and operations described below are unavailable or under verification. Earlier test results refer to their stated revisions; see [current limitations](provisional.md).

![A pickaxe showing 20 of 68 Overslime in the Tinker Station](overslime-station.png)

This Linux test used supplied ingredients and a supplied pickaxe.

The Station and both Anvils have an in-development path for applying and refilling Overslime using source slime ingredients. A slime ball restores 20 points for Earth, 50 for Sky, 60 for Ichor and 80 for Ender. Matching blocks restore more. The operation should consume only what fits and return empty bottles when a supported bottle is used.

Overforced uses four Slimesteel Reinforcements per level, up to five levels, with an upgrade slot for each started level. Partial progress contributes capacity. A Modifier Crystal carries one full level and retains any partial progress when extracted. The source recipe casts a consumed Obsidian Pane with 30 mB of molten Slimesteel on a Casting Table. The source alloy uses molten iron, Sky Slime and seared stone.

Representative Linux tests checked partial and full Overforced application with a LAN guest at each Anvil, including unused input, and a Worktable Crystal extraction and reapplication using a supplied crossbow.

The added shield is spent before ordinary durability on the connected tool, armor and ranged wear paths. Overslime without a friendly modifier has source stat penalties; Overforced is a friend. The durability bar has a blue shield display while shield remains. The Glowing modifier and its special bypass are not implemented.

The source Casting Table recipe fills a glass bottle with 250 mB of the matching slime fluid. The four bottle items and their original artwork are registered, and the source Sky bottle casting recipe is covered by a lightweight catalog test. A supplied Sky bottle was emptied and filled in a Smeltery on Linux, including save/reload and a LAN guest check. A supplied Ichor bottle was consumed in a Linux game and returned glass while applying Levitation and Slowness. Earth, Sky and Ender drinking remains unimplemented because their source effects are not yet connected.

The crafting-table refill has representative Linux normal-click and shift-click checks, including unused bottle and empty bottle returns. Shift-click was also checked for a LAN guest, with supplied tool and bottles.

## Overgrowth inventory recovery candidate

Overgrowth now has an authoritative player-inventory recovery path, including
carried tools, armor and offhand. Once per 20 inventory ticks for each holder,
an eligible tool has a 5% chance per resolved modifier level to restore one
Overslime point. Overworked adds one restored point per level, capped by the
stored capacity. The actively used hand is skipped, so recovery does not
change a bow while it is being drawn. Broken tools are not excluded by this
modifier's source condition. Other custom data is retained.

The implementation requires the exact pinned Overgrowth definition and valid
material, modifier and item-tag snapshots. Missing or changed definitions,
invalid shield data, inactive Overslime, and full shields do not recover.
The phase is transient per holder and world, not saved on the item; switching
worlds can restart the first interval. Non-player holders and custom Overgrowth
module definitions are not handled by this player hook.

Source: [OvergrowthModule](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tools/modules/OvergrowthModule.java)
and [OverslimeModule](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/modules/capacity/OverslimeModule.java).
Six focused Rust rule tests and two Python source/wiring checks cover this
candidate. Full Mixins application, client compilation, gameplay, persistence
and LAN behavior are not yet verified. Earlier evidence below does not validate
this new recovery path.

Linux tests with supplied ingredients checked a Station's partial Overforced application and Overslime refill (20/68), plus shield consumption ahead of Unbreakable on a pickaxe, Longbow, Crossbow and two armor pieces. A LAN guest test checked both armor pieces and their shield changes; a local save/reload retained them. A supplied iron ingot, Sky Crystal, Seared Brick, Obsidian and lava were processed into Slimesteel Reinforcements, applied at the Station, and retained after reload. Natural acquisition of those inputs was not part of that test. The source traits and modifier effects outside these paths remain incomplete.
