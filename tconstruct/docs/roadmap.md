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

The equipment behavior work has source-based head, handle, bow limb/grip, and plating stat calculations in progress. Material attack speed has a Mixins hook but has not been tested in a game. Material-based melee damage, armor protection, bow draw and projectile behavior are not yet connected. The current Mixins version rejects the tested return and after-call hooks in the existing attack and equipment functions; other supported boundaries still need investigation.

## Implementation order

1. Register the original tools, weapons, and armor using the shared source data.
2. Add **pluto lib** as a shared library, initially implementing only Creative tab registration and use by Tinkers' Construct. A mod must be able to register a tab name, icon, ordered items, and material-bearing variants; players must be able to select the tab and obtain its items in the Creative screen. Keep the library in the repository's shared area and Tinkers-specific tab data under `tconstruct/`. Include an illustrated player guide and a minimal example for another mod in the same PR. Item, block, armor, effect, GUI, and inventory APIs are future library scope after Tinkers is complete; do not add placeholder APIs now.
3. Implement tool-specific actions and material traits.
4. Complete the tool station interfaces.
5. Add the remaining blocks.
6. Implement melting, casting, alloys, and their interfaces.
7. Complete acquisition, world generation, and progression.

Each PR updates the relevant user and developer documentation. `main` is the official branch for both Windows and Linux; platform behavior must be verified before claiming it works.

| Area | Current status | Next work |
| --- | --- | --- |
| Part Builder | Source-derived recipes drive a scrolling selection grid, preview and ordinary result pickup for supported inputs. Tested examples include an oak log yielding a wood handle and three planks, an oak plank yielding a wood handle, and limited damageable-item recycling. | Complete material and item support, recycling and Pattern identity; complete material-recipe caching and tag behavior across remaining inputs and reload cases; finish side inventories, remaining click modes, item-specific and non-vanilla custom container behavior, callbacks and detailed information panels. |
| Tinker Station | A shared source-recipe path derives partial base stats and supports three-part tool creation, repair and part replacement. A Linux official build tested the pickaxe lifecycle in both modes and a guest’s Part Builder-to-Station transition; a LAN guest selected and assembled a mattock from command-supplied parts, with its 106-point base maximum durability verified. A crafted mattock was also tested through melee break, repair and save/reload in normal Linux gameplay. | Complete traits/modifiers, combat behavior, layout interactions and naming; verify other tool lifecycles, material combinations and multiplayer. |
| Durability | Pickaxe maximum durability is derived from its selected materials; a representative Linux lifecycle passed in both modes. | Complete traits, original broken behavior, visuals and other tool interactions. |
| Models and tooltips | Tested parts use source material sprites in GUI slots, and tested part tooltips display material names. Original tool textures and static table models are present. | Complete material and variant rendering, 3D models, resource reload, orientation, names, traits and detailed tooltips. |
| Other tools, armor, materials and modifiers | Source definitions, some part IDs and four slimewood plank blocks are present, but complete equipment, material behavior and modifiers are not implemented. | Connect shared stat aggregation, traits, equipment and actions, and expand the supported original definitions. |
| Smeltery and foundry | Not implemented. | Port fluids, melting, casting, alloys and multiblocks. |
| Progression and world content | Not implemented. | Port ores, mobs, books, advancements, recipes and gadgets. |
| Persistence and multiplayer | Tested table inputs, tool name and custom data survive save/reload; representative two-player result races consume inputs once. | Complete original state and component handling, click modes, synchronization and dedicated-server validation. |

The Part Builder uses the original background and a source-recipe 4×4 pattern
list with scrolling, selection and a separate result slot. In both game modes,
representative ordinary crafting, recycling, saving and two-player contention
have been tested. An open table retains its cached recipe across reload until
an input changes; material-recipe caching was tested with a flint value change and an oak-log tag change; other material and tag interactions remain incomplete. Minecraft copper nugget references use the native 26.3 item when present. Minecraft crafting containers use the Horizon 26.3 item registry; item-specific component overrides remain unsupported. Unsupported recipe contexts are rejected. Result Shift-click
works for supported cases with either mouse button. Single 27-slot and 9-slot side containers and 5-slot hoppers have a preliminary shared path; Linux LAN checks covered hopper and dispenser synchronization and contention. The hopper guest displayed five slots in a Linux LAN test. A shulker side inventory was tested with nesting rejected for an in-frame host and LAN guest. Linux LAN tests covered player-to-hopper Shift move and clicks, synchronization and a result race with both halves of a double chest. Other Shift-click paths, drag and pickup-all remain incomplete. The information panels and full visual parity
are also unfinished.

The Tinker Station uses source-derived layouts for supported three-part tools and separate assembly and repair/part-swap icons. The original naming, modifier interface and full layout interactions are incomplete. See the
[station design](station-design.md) and [validation notes](validation.md) for
additional implementation and test details.

The playtest now derives Minecraft recipe and item/fluid tag data from prepared Horizon 26.3 resources rather than requiring a 1.20.1 client. Ordinary sand and red sand each craft four blank casts. These two native inputs are a 26.3 adaptation, not a claim that every original Forge tag or datapack override is supported. Other materials, world reload interactions, official production-binary gameplay and Windows remain to be validated.

CI reuses Cargo dependency and build caches. Runtime, test, configuration and
workflow changes still run the full Mixins transformation and checks. A PR
containing only documentation or image changes since a successful run on the
same base can reuse that result; if the earlier result cannot be established,
CI runs the full checks. Main-branch pushes always run the full checks.
Related implementation changes should be developed in a batch with lightweight
checks first, then share the required full transform and representative game
validation before the verified work is merged to main.

On 2026-10-30, a cold-cache CI run took 23m02s and a subsequent
code-change run with the restored cache took 12m33s. The first client test
build changed from 6m02s to 24.79s. Both runs performed the full Mixins
transformation; timings vary with runner load.

Minecraft’s existing materials, recipes and progression remain unchanged; missing TConstruct content is added alongside them. The scope of full parity and balance will be decided after v0.5.
