# Early book and blank cast recipe-book unlocks

Four existing native recipes now have their original recipe-book unlocks:
Materials and You, the leather-free book substitute, Blank Sand Cast and
Blank Red Sand Cast.

Both book recipes unlock on obtaining a Pattern. Both blank cast recipes
unlock on obtaining a Seared Casting Table, not merely sand or a Scorched
Table. These are the pinned source predicates. Each retains the source OR
recipe-unlocked alternative and rewards only its corresponding recipe. No
achievement display or toast is added; the Materials and You acquisition
advancement is separate and unchanged.

Source advancement paths under Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2` are:

- `src/generated/resources/data/tconstruct/advancements/recipes/misc/common/materials_and_you.json`
- `src/generated/resources/data/tconstruct/advancements/recipes/misc/tables/book_substitute.json`
- `src/generated/resources/data/tconstruct/advancements/recipes/misc/smeltery/{sand_cast,red_sand_cast}.json`

Native recipe IDs flatten source paths, with `sand_cast` and `red_sand_cast`
mapped to existing `blank_sand_cast` and `blank_red_sand_cast`. The source
recipe-unlocked predicate's `recipe` field becomes Horizon's `recipes` field.
All inventory criteria, OR requirements, reward counts and telemetry flags
are otherwise preserved. Exact source fixtures, hashes and attribution are
in `tests/fixtures/early_recipe_unlocks/`.

Existing ingredients, output counts, guide contents and casting behavior
are unchanged. Pattern and Copper Can unlocks remain excluded because their
source inventory criteria require legacy item-tag support. This does not
establish full natural acquisition or current-world recipe-tag parity.

Regression coverage was added for source criteria, native recipe shapes and
ingredients, and package inclusion. Tests were not run, as requested. Brief
static review only; inventory/recipe events, recipe-book UI, crafting,
resource reload, save/reload and LAN remain unverified. No CI or version
checks were performed.
