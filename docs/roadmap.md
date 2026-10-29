# Coverage and roadmap

The reference is Tinkers' Construct 1.20.1, commit
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. The [feature matrix](feature-matrix.md)
links to original sources and records the wider scope. The
[source-to-port record](original-parity.md) describes the evidence and known
mismatches. Archived Wasm work is not evidence of coverage in this port.

The intended v0.5 milestone is a playable prerelease that reproduces almost all
of the original mod's content and behavior on its own. v1.0 is the complete port,
including the original processing and values. Neither milestone has been reached.
Broad compatibility testing with other mods, including hypothetical ID changes,
is not a prerequisite for v0.5. The mod's own saving, synchronization and
transaction integrity remain part of the work.

Implement and verify shared behavior with representative cases, then expand
content from the original definitions. Investigate and test separately where a
source definition introduces distinct behavior or conditions; do not treat
untested content as complete merely because it uses the same framework.

| Area | Current status | Next work |
| --- | --- | --- |
| Part Builder | Source-derived recipes drive a scrolling selection grid, preview and ordinary result pickup for supported inputs. Tested examples include an oak log yielding a wood handle and three planks, and limited damageable-item recycling. | Complete material and item support, recycling and Pattern identity; complete material-recipe caching and tag behavior across the remaining inputs and reload cases; add side inventories, remaining click modes, original container remainders, callbacks and detailed information panels. |
| Tinker Station | A fixed flint/wood pickaxe can be assembled and repaired in a simplified preview. | Port original dynamic layouts, naming, part combinations, component replacement and modifiers. |
| Durability | The fixed pickaxe wears, remains when broken, mines slowly and can be repaired. | Complete original broken behavior, visuals and the remaining tool interactions. |
| Models and tooltips | Tested parts use source material sprites in GUI slots, and tested part tooltips display material names. Original tool textures and static table models are present. | Complete material and variant rendering, 3D models, resource reload, orientation, names, traits and detailed tooltips. |
| Other tools, armor, materials and modifiers | Source definitions and some part IDs are present, but complete equipment, material behavior and modifiers are not implemented. | Connect shared stat aggregation, traits, equipment and actions, and expand the supported original definitions. |
| Smeltery and foundry | Not implemented. | Port fluids, melting, casting, alloys and multiblocks. |
| Progression and world content | Not implemented. | Port ores, mobs, books, advancements, recipes and gadgets. |
| Persistence and multiplayer | Tested table inputs, tool name and custom data survive save/reload; representative two-player result races consume inputs once. | Complete original state and component handling, click modes, synchronization and dedicated-server validation. |

The Part Builder uses the original background and a source-recipe 4×4 pattern
list with scrolling, selection and a separate result slot. In both game modes,
representative ordinary crafting, recycling, saving and two-player contention
have been tested. An open table retains its cached recipe across reload until
an input changes; material-recipe caching was tested with a flint value change and an oak-log tag change; other material and tag interactions remain incomplete. Unsupported recipe contexts are rejected. Result left Shift-click
works for supported cases when no adjacent block entity may supply a side
inventory; original side-inventory behavior, other Shift-click paths, drag and
pickup-all remain incomplete. The information panels and full visual parity
are also unfinished.

The Tinker Station still uses separate assembly and repair icons and a fixed
layout rather than the original dynamic interface. See the
[station design](station-design.md) and [validation notes](validation.md) for
additional implementation and test details.
