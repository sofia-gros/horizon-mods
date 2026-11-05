# Scorched construction recipe-book unlocks

Twelve existing crafting recipes now have their original recipe-book unlocks:
Chiseled Scorched Bricks, Polished Scorched Stone, Scorched Bricks from brick
items, brick slabs/stairs/fence, road slabs/stairs, Ladder, Soul Glass, Soul
Glass Pane and Tinted Glass.

The original item predicates are preserved. Chiseled Bricks unlock on brick
slabs; Polished Stone on Scorched Stone; brick shapes on Scorched Bricks;
road shapes on Scorched Road; Soul Glass Pane on Scorched Soul Glass; and
the remaining recipes on a Scorched Brick. Each criterion is OR with the
original recipe-unlocked alternative, and rewards only its corresponding
existing native recipe. No achievement display or toast is added.

Source files are from Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, beneath
`src/generated/resources/data/tconstruct/advancements/recipes/`:

- `building_blocks/smeltery/scorched/`: `chiseled_scorched_bricks_crafting`,
  `polished_scorched_stone_crafting`, `scorched_bricks_from_brick`,
  `scorched_bricks_{slab,stairs,fence}`, `scorched_road_{slab,stairs}`,
  `scorched_ladder`, `scorched_soul_glass` and `scorched_tinted_glass`.
- `decorations/smeltery/scorched/scorched_soul_glass_pane.json`.

Source recipe IDs are flattened to the existing native IDs. The brick-item
compression recipe keeps its existing `survival_scorched_` prefix. The source
recipe-unlocked predicate's `recipe` field becomes the host's `recipes`
field. All other source data is retained. Exact fixtures, SHA-256 hashes and
attribution are in `tests/fixtures/scorched_decoration_unlocks/`.

Core Scorched machine, Grout/brick-smelting, ordinary Glass/Pane, Lantern,
Lamp, stone-to-bricks and road-smelting unlocks are already present and are
not duplicated. Duct's legacy tag criterion, unregistered machines,
retexturing, stonecutting and alloy/material-dependent Anvil recipes remain
outside this change.

Existing recipes, ingredients/counts, block declarations and machine behavior
are unchanged. Recipe-tag compatibility and full survival acquisition are
not established by adding these unlocks.

Regression coverage was added for exact source criteria, existing recipe
patterns/ingredients/results and package inclusion. Tests were not run, as
requested. Brief static review only; actual inventory events, recipe-book UI,
crafting, resource reload, save/reload and LAN remain unverified. No CI or
version checks were performed.
