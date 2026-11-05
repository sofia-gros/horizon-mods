# Worn Fortune during block breaking

Issue #379. Stacked on the tool Fortune/Fortunate bridge in PR #796. Adds the
Fortune levels from correctly slotted stock Travelers, Plate and Slime armor to
the existing temporary harvesting-tool enchantment view. The harvesting tool
must still be intact and in the live TiC harvest tag, with a valid source
material layout. Vanilla-tool harvesting is not changed.

## Source distinctions

The pinned [Fortune definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/fortune.json)
uses `armor_harvest_enchantment`, with `level: 1` scaling per modifier level,
the four worn slots, and the `modifiable/armor/worn` tag. Fortunate has no such
armor-harvest module, so its worn levels are not converted into Fortune.

[HarvestEnchantmentsModifierHook](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/hook/mining/HarvestEnchantmentsModifierHook.java)
reads each valid equipped tool directly. It does not use the equipment iterator
that filters broken armor. Therefore **broken worn armor still contributes
Fortune** here. This intentionally differs from the broken-filtered armor
Looting path. Creative players do not receive the armor-harvest additions.

[ToolHarvestLogic.runBlockBreak](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/helper/ToolHarvestLogic.java)
temporarily applies the worn enchantments for normal block breaking and its area
loop, then restores the original enchantment tag. This port leaves inventory
untouched and adds each piece's resolved material-trait plus upgrade level once
to the transient native loot tool. Existing mainhand Fortune/Fortunate and native
Fortune enchants are retained, with checked arithmetic and the existing total
limit of 255.

[HarvestModule](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tools/modules/interaction/HarvestModule.java)
calls `Block.getDrops` for right-click crops without the armor-enchantment hook.
The crop adapter now calls an explicit tool-only loot entry. This preserves
PR #796's constant tool enchantments without leaking armor Fortune into crop
replanting. Its roll retains the native location, block-entity data, random
sequence, dynamic drops and native loot callback, including the no-projection
fallback. It does not invoke other mixin HEAD hooks on `block_loot_at`.

## Boundaries and review

- Worn pieces require the correct stock slot and live modifiable/worn tags.
  Count-one and non-opaque stacks are required. Source layouts and active
  Fortune definitions are read live; malformed or unknown participating data
  declines the whole projection rather than guessing a partial bonus.
- Ordinary and existing area block breaking use this path. The source takes
  one armor snapshot around its entire area operation; this bounded port reads
  the armor on each actual block loot roll. Equipment changes from intervening
  third-party callbacks need further integration work.
- Held shields, armor in hands, Fortunate armor effects, Luck, fishing,
  acquisition/extraction, arbitrary custom tools/definitions and non-player
  harvesting remain separate.
- Static review covered the source's broken/creative handling, crop versus
  block-break entry points, additive level calculation, clone lifetimes and
  unchanged native roll semantics. Two pure regression tests were added but
  not run. The existing harness includes them without new dependencies.
- No tests, builds, planner, CI inspection, version checks or merges were run.
  Actual distributions, combined armor/tool behavior, reload/save, callback
  composition and LAN remain for final integration validation.
