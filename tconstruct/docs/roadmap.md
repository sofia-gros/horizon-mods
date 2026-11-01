# Coverage and roadmap

The reference is Tinkers' Construct 1.20.1, commit
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. The [feature matrix](feature-matrix.md)
links to original sources and records the wider scope. The
[source-to-port record](original-parity.md) describes the evidence and known
mismatches. Archived Wasm work is not evidence of coverage in this port.

The intended v0.5 milestone is a playable prerelease that reproduces almost all
of the original mod's content and behavior on its own. The scope of full parity and balance will be decided after v0.5. Existing Horizon 26.3 Minecraft materials, recipes and progression remain unchanged. v0.5 has not been reached.
Broad compatibility testing with other mods, including hypothetical ID changes,
is not a prerequisite for v0.5. The mod's own saving, synchronization and
transaction integrity remain part of the work.

Implement and verify shared behavior with representative cases, then expand
content from the original definitions. Investigate and test separately where a
source definition introduces distinct behavior or conditions; do not treat
untested content as complete merely because it uses the same framework.

Material attack damage, attack speed, and armor attributes have been tested with representative equipment on Linux. A longbow can be drawn and fired using the existing bow behavior; switching hands cancels its shot. Material draw speed for longbows and crossbows, along with crossbow charging and firing, has been tested with representative values on Linux. Command-supplied wood-material bows were tested for survival wear, broken retention and use rejection, limb-material repair, save/reload, and Creative wear exemption. Representative Longbow and Crossbow arrows now use material velocity and accuracy, including a tested limb exchange and LAN shots. Firework rockets and conditional projectile-stat modifiers remain unfinished. Most modifier effects remain unimplemented. Vanilla weapons can currently heal targets under sufficiently strong Weakness in the underlying game; material weapons clamp their basic damage to zero.

A Creative block tab now lists the registered block items. Linux tests covered representative placement and LAN synchronization, a slimewood door, a seared brick drop and save/reload, and the appearance of the three layered slime-metal blocks. Connected glass has representative Linux and LAN checks; source slime bounce and piston behavior, other machine operation, and remaining blocks are still incomplete. The [block registration guide](block-registration.md) describes the limits.

## v0.5 implementation order

Each stage is split into playable PRs with shared source-derived behavior and representative tests. Registration, working behavior, and individual verification are different milestones; untested variants are not automatically complete. Existing Horizon vanilla resources and progression are retained.

1. **Material traits:** expand the existing attack, durability and repair dispatcher from original definitions. Accept source-value and level checks, part replacement, save/reload, representative combat and LAN. Representative melee stats, mining effects and Dense wear/repair are implemented and tested; other trait modules remain open.
2. **Equipment actions:** implement original tool, weapon and armor actions, area mining, projectiles and armor effects. Accept representative use, wear, save/reload and multiplayer by shared behavior type.
3. **Modifier and part processing:** implement the higher-tier Station and survival assembly for four-part tools, then expand Station modifier recipes, crystals, parts and materials, and dyeing. Accept source ingredients and slots, partial progress, outputs, effects and persistence.
4. **Smelting, casting and remaining blocks:** implement special recipes, remaining containers, transport/automation and source-specific block actions. Accept complete input-to-output loops with conservation, save/reload and LAN.
5. **Acquisition and world:** add necessary ores, slime terrain, creatures and resources while reusing appropriate vanilla content. Accept survival acquisition and world persistence.
6. **Progression and remaining content:** implement guidebook, advancements, gadgets and remaining original feature-matrix entries. Accept a coherent progression through their normal prerequisites.
7. **Release validation:** run a new survival playthrough, save/reload and LAN, and verify Linux and Windows startup before release claims.

GUI polish, including Station naming and representative connected-glass rendering, is already available; Station preview trim remains blocked pending a supported atlas preparation boundary. Other unfinished GUI or source behavior remains tracked in the [GUI review](gui-review.md), rather than being declared complete. Broader PlutoLib development is deferred until v1.0 or later.

Each PR updates the relevant user and developer documentation. `main` is the official branch for both Windows and Linux; platform behavior must be verified before claiming it works.

| Area | Current status | Next work |
| --- | --- | --- |
| Part Builder | Source-derived recipes drive a scrolling selection grid, preview and ordinary result pickup for supported inputs. Tested examples include an oak log yielding a wood handle and three planks, an oak plank yielding a wood handle, and limited damageable-item recycling. | Complete material and item support, recycling and Pattern identity; complete material-recipe caching and tag behavior across remaining inputs and reload cases; finish side inventories, remaining click modes, item-specific and non-vanilla custom container behavior, callbacks and detailed information panels. |
| Tinker Station | A shared source-recipe path derives partial base stats and supports three-part tool creation, repair and part replacement. A Linux official build tested the pickaxe lifecycle in both modes and a guest’s Part Builder-to-Station transition; a LAN guest selected and assembled a mattock from command-supplied parts, with its 106-point base maximum durability verified. A crafted mattock was also tested through melee break, repair and save/reload in normal Linux gameplay. | Complete traits/modifiers, combat behavior and remaining layout interactions; verify other tool lifecycles, material combinations and multiplayer. |
| Durability | Pickaxe maximum durability is derived from its selected materials; a representative Linux lifecycle passed in both modes. | Complete traits, original broken behavior, visuals and other tool interactions. |
| Models and tooltips | Tested parts use source material sprites in GUI slots, and tested part tooltips display material names. Original tool textures and static table models are present. | Complete material and variant rendering, 3D models, resource reload, orientation, names, traits and detailed tooltips. |
| Other tools, armor, materials and modifiers | The 45 equipment IDs are registered; representative icons and armor placement were tested. Some shared material stats and traits work, but equipment actions and most modifiers remain incomplete. | Expand shared stats and traits, then implement tool-specific actions and the remaining modifiers. |
| Smeltery and foundry | A small Smeltery casts amethyst bronze; a small Foundry melts copper ore and retains a gold byproduct. Both representative paths were tested on Linux, including LAN. | Expand beyond representative standard bucket transfers to other containers, recipes, larger structures, automation and machine visuals. |
| Progression and world content | Not implemented. | Port ores, mobs, books, advancements, recipes and gadgets. |
| Persistence and multiplayer | Tested table inputs, tool name and custom data survive save/reload; representative two-player result races consume inputs once. | Complete original state and component handling, click modes, synchronization and dedicated-server validation. |

The Part Builder uses the original background and a source-recipe 4×4 pattern
list with scrolling, selection and a separate result slot. In both game modes,
representative ordinary crafting, recycling, saving and two-player contention
have been tested. An open table retains its cached recipe across reload until
an input changes; material-recipe caching was tested with a flint value change and an oak-log tag change; other material and tag interactions remain incomplete. Minecraft copper nugget references use the native 26.3 item when present. Minecraft crafting containers use the Horizon 26.3 item registry; item-specific component overrides remain unsupported. Unsupported recipe contexts are rejected. Result Shift-click
works for supported cases with either mouse button. Single 27-slot and 9-slot side containers and 5-slot hoppers have a preliminary shared path; Linux LAN checks covered hopper and dispenser synchronization and contention. The hopper guest displayed five slots in a Linux LAN test. A shulker side inventory was tested with nesting rejected for an in-frame host and LAN guest. Linux LAN tests covered player-to-hopper Shift move and clicks, synchronization and a result race with both halves of a double chest. Other Shift-click paths, drag and pickup-all remain incomplete. Information panels are implemented for supported cases; remaining source visual and interaction differences are tracked in the GUI review.

The Tinker Station uses source-derived layouts for supported three-part tools and separate assembly and repair/part-swap icons. Naming and representative modifier and layout interactions work; other modifiers and exact source parity remain incomplete. See the
[station design](station-design.md) and [validation notes](validation.md) for
additional implementation and test details.

The playtest now derives Minecraft recipe and item/fluid tag data from prepared Horizon 26.3 resources rather than requiring a 1.20.1 client. Ordinary sand and red sand each craft four blank casts. These two native inputs are a 26.3 adaptation, not a claim that every original Forge tag or datapack override is supported. Other materials and world reload interactions remain to be validated; representative production-binary gameplay passed on Linux, while Windows remains unverified.

CI reuses Cargo dependency and build caches. Run the Mixins validation workflow
manually for the final commit of a change; it performs the full checks. Verify
the run's head SHA and successful result before merging. Ordinary pushes do not
start a build. See the [development guide](../../docs/development.md).
Related implementation changes should be developed in a batch with lightweight
checks first, then share the required full transform and representative game
validation before the verified work is merged to main.

On 2026-10-30, a cold-cache CI run took 23m02s and a subsequent
code-change run with the restored cache took 12m33s. The first client test
build changed from 6m02s to 24.79s. Both runs performed the full Mixins
transformation; timings vary with runner load.

Minecraft’s existing materials, recipes and progression remain unchanged; missing TConstruct content is added alongside them. The scope of full parity and balance will be decided after v0.5.

The material trait prototype resolves part traits from the active world data. Cultivated and the repair portion of Preserved affect material repair; the melee portions of Jagged and Stonebound affect attack damage. These effects were tested with representative materials on Linux, including a part replacement and save/reload. Preserved durability and Ductile durability, melee damage, and armor toughness were tested on Linux for newly generated tools. The station test used a temporary datapack to isolate the traits. Previously saved tools do not yet refresh their maximum durability automatically. Representative mining portions of Jagged, Stonebound, Lightweight, Heavy, Vintage, Entangled and Maintained, and Dense repair/wear are implemented and tested on Linux. Ductile projectile damage, other trait modules, applied modifiers beyond Sharpness and Swiftstrike, and other ranged trait effects remain unfinished.

The Station supports partial Sharpness and Swiftstrike recipes, upgrade slots, and melee stat changes. Linux single-player tests covered partial and completed upgrades, leftover delivery, slot and ingredient rejection, combat, part replacement, and save/reload. A LAN guest also completed a partial and full Sharpness upgrade, including leftover delivery. Modifier crystals and other applied modifiers remain unfinished.
