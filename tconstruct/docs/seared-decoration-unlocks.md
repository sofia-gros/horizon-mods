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

Added regression coverage compares all twenty definitions with source,
checks rewarded recipes' patterns/results and verifies packaged files.
Tests were not run for this change. Brief static review only: actual inventory
events, recipe-book display, crafting, resource reload, save/reload and LAN
remain unverified. No CI or version checks were performed.
