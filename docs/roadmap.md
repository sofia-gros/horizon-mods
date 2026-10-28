# Coverage and roadmap

The reference is Tinkers' Construct 1.20.1, commit
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. The [feature matrix](feature-matrix.md)
links the original source and lists the remaining scope. Archived Wasm work is
not evidence of coverage here.

| Area | Current status | Next work |
| --- | --- | --- |
| Part Builder | Playable fixed flint/wood subset | Original recipe-derived lists, output tooltip, other materials and variants |
| Tinker Station | Fixed pickaxe assembly and flint repair | Naming UI, all combinations, component replacement and modifiers |
| Durability | Wear, slow broken mining and repair for fixed pickaxe | Full original broken behavior and original broken textures |
| Models and tooltips | Original textures for the fixed tool and parts; static table models | All material combinations, orientation, tooltips and traits |
| Other tools, armor, materials and modifiers | Not implemented | Definition-driven stat aggregation, traits, equipment and actions |
| Smeltery and foundry | Not implemented | Fluids, melting, casting, alloys and multiblocks |
| Progression and world content | Not implemented | Ores, mobs, books, advancements, recipes and gadgets |
| Persistence and multiplayer | Station input, tool name/custom data and LAN races tested | Dedicated server, arbitrary components, more click modes and mixed-mod testing |

The GUI uses the original backgrounds. Three fixed part buttons select a
preview and the result is taken from the separate result slot; the original
recipe-dependent lists and information panels are not implemented. Assembly
and repair have separate icons, unlike the original dynamic layouts. Shift-click,
drag and pickup-all are disabled in these screens. Only oak planks work for
the wood parts. See [station design](station-design.md) and
[validation](validation.md) for authority, save and test details.
