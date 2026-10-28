# Tinkers’ Construct port roadmap

The goal is the full original 1.20.1 Mod, including its presentation, values,
processing, and behavior. Each completion unit must trace the fixed original
source and data to implementation and tests; no prototype is accepted merely
because it looks similar. The inventories below identify source scope, not
implemented coverage.

| Unit | Original reference | Status on this foundation |
| --- | --- | --- |
| Part Builder, recipes and material selection | `tables/`, `library/recipe/partbuilder/`, `data/tconstruct/recipes/tools/parts/builder` | Not implemented here. Preview in PR #13. |
| Tinker Station and related workstations | `tables/`, `tinkering/station_layouts/` | Not implemented here. |
| Materials, variants, parts, tools, weapons and armor | `library/materials/`, `library/tools/`, `tools/`, `tinkering/materials/` | Not implemented here. |
| Modifiers, traits and interactions | `library/modifiers/`, `tools/modifiers/`, `tinkering/modifiers/` | Not implemented here. |
| Smeltery, foundry, casting, fluids and multiblocks | `smeltery/`, `fluids/`, `recipes/smeltery/` | Not implemented here. |
| Recipes, progression, world generation and loot | `recipes/`, `data/`, `world/`, `loot/`, advancements and tags | Not implemented here. |
| Models, textures, animations, sound, translations and tooltips | `assets/tconstruct/`, client rendering and screen code | Not implemented here. |
| Save/reload, synchronization, automation and multiplayer | original block entities, menus, packets and inventory code | Not implemented here. |

The [source inventory](upstream-inventory.json) lists selected generated
registrations, [complete tracked tree](upstream-tree.txt.gz) lists all upstream
paths, and [recipe catalog](upstream-recipes.json.gz) preserves the referenced
material, Part Builder and station-layout JSON. Their presence does not mean
those contents are implemented or exhaustively reviewed. Additional original
features discovered in those files remain in scope.
