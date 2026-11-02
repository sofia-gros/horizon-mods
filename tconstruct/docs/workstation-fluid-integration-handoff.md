# Workstation and fluid integration handoff

This candidate merges the fixed material integration #309 (`0c04d101a604374be7a068b933bcfdc45a227f37`) into the fixed workstation/fluid candidate #320 (`4e24f01322838a16af95755133d6d3af782a65d8`). These are both merge parents; their fixed heads remain unchanged.

| Workstation / fluid series | Adoption in this candidate | Verification boundary |
| --- | --- | --- |
| #298, #302, #303, #305 | Inherited through #320: unified splash aliases, restricted recipe and core effects, bounded guest cures and guest transaction | Apply, type and game results belong to their exact tested heads; inherited code is not a new runtime pass |
| #308, #313, #315, #317 | Inherited through #320: splash and lingering bottle registration/brewing, source/cauldron use, conflict guard and waterlogged selection | Natural acquisition, cauldron and world interactions require official target tests |
| #310 / #268 | Named `Ingredient.items` in `anvil-material-recipe.rs` | Required by newer Horizon ingredient shape; official type-check of this combined candidate remains pending |
| #314 | Equivalent ordinary-comment header correction inherited via #315 | #314's fixed head remains separate |
| #320 | Header static checker and historical audit evidence | Run checker against the selected final candidate and new changes |
| #309 | Material/armor and upgrade logic merged with fluid series | Combined candidate requires dedicated apply, type, behavior, and package verification |

The one textual conflict was `source-upgrade-rules.rs`: this candidate retains #309's speedy/step-up and gilded slot and level handling together with splashing's allowed tools, ability accounting, fluid levels and recipe handling. Other automatic merges need the same formal review and validation as the full candidate. No claim of completion or gameplay pass is implied.
