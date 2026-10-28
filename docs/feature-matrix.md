# Tinkers' Construct feature coverage

Reference: [1.20.1 source at f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2). The
[upstream inventory](upstream-inventory.json) records every generated material
definition (104 files), tool definition (45), and modifier definition (308)
at that revision. Those file counts include compatibility and variants; they are
not counts of fully playable features. Every entry is unimplemented except the
narrow prototype called out below.

| Area | Reference | Current coverage and gaps |
| --- | --- | --- |
| Materials and parts | [material definitions and stats](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/materials), [part recipes](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/recipes/tools/parts) | Flint head and wood binding/handle IDs only; material-bearing component state, variants and all other parts/materials are missing. The temporary command costs differ from the original part recipes. |
| Tools and armor | [tool definitions](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/tool_definitions) | Fixed flint/wood pickaxe prototype only; component combinations, other tools, ranged weapons, armor and equipment are missing. |
| Traits and modifiers | [modifier definitions](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers) | Not implemented, including material traits, upgrade and ability slots, interactions and leveling. |
| Tables and GUIs | [tables source](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tables) | Part Builder and Tinker Station blocks, menus, pattern selection, transactions, repair and naming are not implemented. |
| Smeltery and casting | [smeltery source](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery), [recipes](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/recipes/smeltery) | Not implemented: multiblocks, tanks, fluids, melting, alloys, fuels, casts, pouring and casting. |
| Crafting and progression | [recipe groups](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/recipes), [world content](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/world) | Normal recipes, advancement, books, ores, slimes, world features and gadgets are missing. |
| Save and multiplayer | [tool state](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/nbt), [station recipes](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tables/recipe) | Prototype item ID and damage survive an in-frame save/reload. Names, custom data, persistent station inventories, conflict-safe transactions, LAN and dedicated server remain unverified or unimplemented. |

Source inventory is not exhaustive of behavior expressed in Java, recipes,
assets, integrations or tags; the linked source trees remain the authority.
The next unit must close the whole station interaction loop before marking it
supported. Preserve unrelated item components on repair, and commit input and
output as one host-authoritative operation to prevent duplication or loss.
