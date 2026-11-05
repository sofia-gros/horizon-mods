# Part-swap workstation capacity

The fixed [TinkerStationPartSwapping.getValidatedResult](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tables/recipe/TinkerStationPartSwapping.java) rejects a swap when the tool definition's total number of parts exceeds the workstation's input count. A single replacement part does not make a large tool eligible for a smaller table.

The shared source Station planner now checks the current definition before rebuilding a swapped tool or returning a consumption plan. Three-part Pickaxes remain eligible at a three-input Station or five-input Anvil. Four-part Sledge Hammers require the Anvil. Even a tool-building recipe with a one-part override cannot bypass the definition-based capacity check. The existing building-recipe part selection, repair amounts, modifier validation and actual transaction commit paths are otherwise unchanged.

Unit tests exercise 1–6 total parts, both supported workstation capacities, invalid capacities, malformed/unsupported/duplicate tool-parts hooks, and a recipe override. Source archive tests verify the stock Pickaxe/Sledge Hammer counts, and wiring checks locate the guard before rebuild/consumption. The Mixins workflow runs the production recipe-module tests after full apply and client typechecking.

Actual GUI preview, input removal, Shift operations, Anvil transactions, world reload and LAN remain unverified for this change. A capacity failure returns no swap plan and supplies diagnostic text through the existing planner interface; its visible presentation and localization are not newly verified. This does not change casting-based swaps or add the remaining source swap features.
