# Clear tinted glass candidate

At horizon-mods `2f022395`, the pinned TConstruct `f4e77d43` item tag `forge:glass/tinted` requires `tconstruct:clear_tinted_glass`. Its absence made the entire tag unresolved even when Forge supplied vanilla tinted glass. This candidate registers the original block and block item using Horizon's `minecraft:tinted_glass` template, adds self-drop loot, and imports the pinned source base texture. It does not enable connected-texture rendering; that remains separate follow-up work. It does not modify the original tag or make required entries optional. Other packs' tag contributions and missing required entries still follow the existing resolver.

The retained original `smeltery/casting/amethyst/glass` recipe outputs this block, consumes one `forge:glass/colorless` cast, costs 200 mB molten amethyst and cools for 97 ticks. Registering the output allows that existing recipe to target a registered item; it does not add a separate recipe. The source tag can now include both registered vanilla tinted glass and the new item, subject to normal pack layering and live registry state. `forge:wither_bones` is separate and remains unresolved if Necrotic Bone is unavailable.

A pre-existing local host binary prepared a resource tree successfully and produced both a block and BlockItem with this identifier, retaining the source casting recipe. The exact candidate mixins were not built into that binary. Scoped tests cover source recipe constants, the base asset, and vanilla template choice. Host execution and Rust type-checking for this exact candidate remain unproven. They do not establish actual light blocking, connected-face rendering, survival drops, fluid/heat consumption, save/reload, multiplayer or the exact integrated build. Verify those in SP and LAN, including both tinted-glass casts and user pack overrides, before calling this complete.

The base texture matches the GitHub blob ID at the pinned `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` revision. An earlier package containing all 16 textures was inspected, including its byte-identical TConstruct MIT license. The reduced candidate has also passed a package test for the base texture, absence of connected variants, and included MIT license. It has not been run in the host; packaging does not establish integrated compilation, light blocking, or live rendering.

The vanilla tinted-glass template is intended to provide light blocking. Template selection alone does not prove propagation or shading in this host. No modifications to connected-texture or atlas helpers are needed for this partial registration and casting-data candidate.

The Seared and Scorched tinted glass registrations also use the vanilla tinted-glass template, with the pinned source hardness, resistance, glass sound, tool requirement, and existing self-drop loot. This follows the pinned source behavior that blocks sky and block light. The generator and committed declarations are checked together. Integrated host light propagation, block rendering, save/reload, and LAN have not been tested. Connected-face rendering remains separate.

Existing worlds may retain previously saved light in chunks already marked lit.
The pinned host can reuse saved light without automatically recalculating it
when a block template changes. The correction therefore may require relighting;
a reliable migration procedure and live verification remain open. Scorched's
source map color also remains unmatched by the tinted-glass template.
