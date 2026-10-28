# Coverage and roadmap

The reference is Tinkers' Construct 1.20.1, commit
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. Status refers only to this Mixins
implementation; the archived Wasm work is not evidence of coverage here.

| Area | Status | Next work |
| --- | --- | --- |
| Material and part creation | Prototype, verified in single-player | Flint and wood only; add patterns, recipes and Part Builder UI |
| Pickaxe assembly | Prototype, verified in single-player | Replace command flow with Tinker Station; persist actual component state |
| Mining and durability | Prototype, verified in single-player | Extend broken-state parity, repair and traits |
| Rendering and tooltip | Fixed-material original textures, verified in single-player | Add material-specific combinations, broken appearance and tooltips |
| Other tools, materials and modifiers | Not implemented | Add definitions, stat aggregation, traits and modifiers with reference tests |
| Stations and crafting | Not implemented | Part Builder, Tinker Station, recipes, repair and modification |
| Smeltery and foundry | Not implemented | Fluids, melting, casting, multiblocks and interaction |
| Progression, world content and compatibility | Not implemented | Ores, mobs, books, advancements, networking and interoperability |

Extend the repeatable game test to cover additional paths and replace shortcuts with real part state and UI before broadening material and
tool coverage. Dedicated server and multiplayer behavior have not been tested.
