# Tinkers' Construct feature coverage

[Original parity tracking and Part Builder source crosswalk](original-parity.md). This matrix is an incomplete audit, not a comprehensive list of implemented or unimplemented features.

Reference: [1.20.1 source at f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2). The
[upstream inventory](upstream-inventory.json) records every generated material
definition (104 files), tool definition (45), and modifier definition (308)
at that revision. Those file counts include compatibility and variants; they are
not counts of fully playable features. Every entry is unimplemented except the
narrow prototype called out below.

| Area | Reference | Current coverage and gaps |
| --- | --- | --- |
| Materials and parts | [material definitions and stats](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/materials), [part recipes](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/recipes/tools/parts) | The Part Builder makes a flint head and wood handle/binding; material-bearing component state, variants and all other parts/materials are missing. The legacy diagnostic command has different costs from the original recipes. |
| Tools and armor | [tool definitions](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/tool_definitions) | Fixed flint/wood pickaxe prototype only; component combinations, other tools, ranged weapons, armor and equipment are missing. |
| Traits and modifiers | [modifier definitions](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers) | Not implemented, including material traits, upgrade and ability slots, interactions and leveling. |
| Tables and GUIs | [tables source](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tables) | The two placed blocks have original backgrounds and a limited pickup GUI for fixed-part assembly and flint repair. Part icons directly take output rather than selecting a recipe and using the original result slot. Naming, full selection, modifiers and other materials are missing. |
| Smeltery and casting | [smeltery source](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery), [recipes](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/recipes/smeltery) | Not implemented: multiblocks, tanks, fluids, melting, alloys, fuels, casts, pouring and casting. |
| Crafting and progression | [recipe groups](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/recipes), [world content](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/world) | Pattern and table recipes are available; other recipes, advancement, books, ores, slimes, world features and gadgets are missing. |
| Save and multiplayer | [tool state](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/nbt), [station recipes](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tables/recipe) | Station inputs and the fixed tool survive save/reload; repair retains its name and custom data. Two-player LAN contention was tested for part creation, assembly and repair. Dedicated server, arbitrary components and broader concurrency remain unverified. |

Source inventory is not exhaustive of behavior expressed in Java, recipes,
assets, integrations or tags; the linked source trees remain the authority.
The fixed-material station loop is a subset, not complete station parity.
Broaden material and tool coverage while preserving item components and
host-authoritative input/output transactions.
