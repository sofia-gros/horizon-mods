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

## Current intake and verification

This adopts the four resource files and their pinned fixtures from PR #751,
head `f39c02aed87fa19216b986976c5be6acb3c597d7`, onto ordinary
`9e032ef5b013487ea08b997eb08df84825fb09ba`. The four recipes already exist
unchanged. All 64 existing advancement resources in that base were read by
exact Git blob: none rewards these four recipes, so this adds missing unlocks
rather than a second grant path.

This is TConstruct progression data using the existing native advancement
loader, inventory criteria and recipe rewards. It does not require another
Pluto registry, component-event opt-in or callback. Pluto's loaded-goal
projection is not used to create new IDs. The existing generic mechanics
remain unchanged. Native 41f's Tree::load_with reads singular `advancement/`,
its recipe criterion and reward code read `recipes`, and its award path uses
the existing recipe index. These are source checks, not runtime proof.

Two existing resource regressions pass: exact upstream criteria/OR/reward
adaptation and unchanged native recipe inputs/results/counts. The existing
full content-package test is retained, but was not run in this bounded sparse
intake. Only its import was moved into that test so resource-only checks do
not require unrelated packager dependencies. No test assertions were removed.

Reproduce the two checks from `tconstruct/tests`:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest -v \
  test_early_recipe_unlocks.EarlyRecipeUnlocks.test_source_inventory_conditions_and_or_native_recipe_rewards \
  test_early_recipe_unlocks.EarlyRecipeUnlocks.test_existing_native_inputs_results_and_counts_match_source
```

Full package inclusion, actual inventory/recipe events, recipe-book UI,
crafting, resource reload, save/reload and LAN remain unverified. There was
no Rust, Cargo, CI, plan, apply or game execution for this delta. The intake
receipt is `integration/recipe-intake-20261107/early-unlocks.json`.
