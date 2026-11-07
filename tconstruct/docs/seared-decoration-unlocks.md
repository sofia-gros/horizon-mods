# Seared construction recipe-book unlocks

Twenty existing native recipes now have their original recipe-book unlock
advancements:

- Seared Bricks from brick items; brick and cobble slabs, stairs and walls;
  stone and paver slabs and stairs; Fancy Seared Bricks; and Seared Ladder.
- Seared Glass, Soul Glass and Tinted Glass, plus regular and Soul Glass panes.
- Seared Lantern and Seared Casting Tank.

Each preserves the pinned source's exact item acquisition criterion and OR
recipe-unlocked alternative. Most construction shapes unlock on their full
block; Fancy Seared Bricks unlock on brick slabs; panes unlock on their glass
block. The remaining recipes unlock on a Seared Brick. Rewards point only to
the corresponding existing native recipe. These are hidden recipe-book
advancements, with no display or achievement toast.

Source files are from Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, beneath
`src/generated/resources/data/tconstruct/advancements/recipes/`:

- `building_blocks/smeltery/seared/`: `seared_bricks_from_brick`,
  `seared_bricks_{slab,stairs,wall}`, `seared_cobble_{slab,stairs,wall}`,
  `seared_fancy_bricks_crafting`, `seared_glass`, `seared_ladder`,
  `seared_paver_{slab,stairs}`, `seared_soul_glass`,
  `seared_stone_{slab,stairs}`, and `seared_tinted_glass`.
- `decorations/smeltery/seared/`: `lantern`, `seared_glass_pane`, and
  `seared_soul_glass_pane`.
- `redstone/smeltery/seared/seared_casting_tank.json`.

Source recipe IDs are flattened to the existing native IDs. In particular,
`seared_bricks_from_brick` and `seared_glass` have the `survival_seared_`
prefix, and `lantern` becomes `seared_lantern`. The source recipe-unlocked
predicate's `recipe` field becomes the host's `recipes` field. No other
criterion, requirement or reward is changed. Exact source fixtures, SHA-256
hashes and attribution are in `tests/fixtures/seared_decoration_unlocks/`.

This does not duplicate the eleven existing core Seared machine unlocks,
Grout/brick-smelting unlocks, lamp unlock or stone-processing unlocks. Duct
and Fluid Cannon unlocks remain excluded because their source criteria use
legacy item tags that need current-world tag handling. Stonecutting recipes
and retextured machine recipes are outside this change.

No crafting ingredients, output counts, block registrations or machine
behavior change. Existing recipe tag compatibility limits remain, including
fixed iron input for the Lantern. A recipe-book unlock does not establish
that every ingredient or machine operation is available in survival.

## Current intake and verification

This is the exact own-resource scope of PR #735, head
`b3d99db71003966ca83b1c9cee910571667836ef`, adapted onto ordinary
`18c2d21059ae55549391388991306343b6ca3617`. It does not import the historical
branch tree. All twenty rewarded crafting recipes and their registered block
outputs already exist in that ordinary base and remain unchanged.

Pluto owns shared compatibility and reusable advancement/recipe mechanics;
these item criteria and recipe IDs are TConstruct progression data. The
existing native advancement resource loader, item-ID inventory predicate and
recipe reward path are sufficient. No new component opt-in, registry, hook,
public API or loaded-goal projection is needed for these new resource IDs.
Existing generic Pluto and native fallback behavior stays unchanged.

The two existing resource tests passed on this source snapshot: all twenty
pinned definitions/OR criteria/rewards, and native patterns/results/counts
plus registered output declarations. The third, full content-package test is
retained but not run in this bounded sparse intake. Its import alone moved
into the test method; all original assertions remain. From `tconstruct/tests`:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest -v \
  test_seared_decoration_unlocks.SearedDecorationUnlocks.test_exact_source_inventory_criteria_and_or_recipe_rewards \
  test_seared_decoration_unlocks.SearedDecorationUnlocks.test_rewarded_native_patterns_results_and_registered_outputs
```

Full package inclusion, native inventory/recipe events, recipe-book display,
crafting, reload, save/reload and LAN remain unverified for this delta. There
was no Cargo, Rust, plan, apply, CI or game execution. The exact intake and
remaining gates are recorded in
`integration/recipe-intake-20261107/seared-unlocks.json`.
