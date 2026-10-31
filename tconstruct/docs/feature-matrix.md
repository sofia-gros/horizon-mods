# Tinkers' Construct feature coverage

[Original parity tracking and Part Builder source crosswalk](original-parity.md). This matrix is an incomplete audit, not a comprehensive list of implemented or unimplemented features.

Reference: [1.20.1 source at f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2). The
[upstream inventory](upstream-inventory.json) records every generated material
definition (104 files), tool definition (45), and modifier definition (308)
at that revision. Those file counts include compatibility and variants; they are
not counts of fully playable features. Coverage varies by feature: the table distinguishes representative tested paths from features that are still incomplete.

| Area | Reference | Current coverage and gaps |
| --- | --- | --- |
| Materials and parts | [material definitions and stats](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/materials), [part recipes](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/recipes/tools/parts) | The Part Builder uses source part recipes and material tags. Representative part creation and LAN storage were tested; all material and part combinations have not been individually verified. |
| Tools and armor | [tool definitions](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/tool_definitions) | 45 equipment IDs are registered. Representative material-based melee, armor, longbow and crossbow behavior was tested; individual weapons, materials and their unique abilities are not all complete or verified. |
| Traits and modifiers | [modifier definitions](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers) | Selected material traits and the Sharpness and Swiftstrike upgrades have working paths and representative tests. Other traits, modifiers and abilities remain incomplete. |
| Tables and GUIs | [tables source](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tables) | Part Builder and Tinker Station use source layouts, recipes and information panels for supported operations. Linux tests covered naming, two- and three-part assembly, repair, replacement, two upgrades and LAN storage. Other recipes, modifiers and exact visual parity remain incomplete. |
| Block registration | [block guide](block-registration.md) | The Creative block tab and representative building-block placement, LAN synchronization, save/reload and a mined drop were tested on Linux. Connected glass and panes have representative rendering and LAN tests. Many other blocks and source-specific behaviors remain incomplete. |
| Smeltery and casting | [smeltery source](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery), [recipes](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/recipes/smeltery) | The Melter copper, small Smeltery alloy and small Foundry copper-ore paths work in Linux single-player and LAN. Representative standard bucket transfers have been tested on Linux and LAN; other containers, recipes, structure behavior and machine visuals remain incomplete or unverified. |
| Crafting and progression | [recipe groups](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/recipes), [world content](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/world) | Pattern and table recipes are available; other recipes, advancement, books, ores, slimes, world features and gadgets are missing. |
| Save and multiplayer | [tool state](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/nbt), [station recipes](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tables/recipe) | Station inputs and representative material-based tools survive save/reload; repair retains its name and custom data. Two-player LAN contention was tested for part creation, assembly and repair. Dedicated server, arbitrary components and broader concurrency remain unverified. |

Source inventory is not exhaustive of behavior expressed in Java, recipes,
assets, integrations or tags; the linked source trees remain the authority.
The tested station loop is a subset, not complete station parity.
Broaden material and tool coverage while preserving item components and
host-authoritative input/output transactions.
