# Survival progression check

This route connects the existing recipes from gathered vanilla resources to a
Tinker Station, Melter, Smeltery and alloy Anvil. The data checks find no missing
recipe in this route. **A continuous, fixture-free playthrough is still pending.**
The existing `tests/game/survival_chain.py` supplies vanilla materials and uses
commands; its results do not establish natural acquisition or survival completion.

Recipes below come from the bundled TConstruct 1.20.1 recipe archive, pinned to
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. Native crafting adaptations are in
`content/data/tconstruct/recipe/`; machine recipes are in
`content/reference/tconstruct-recipes.zip`. No substitute recipes are needed.

## Start with gathered resources

Create a fresh Survival world with the matching PlutoLib and TConstruct bundles.
Record their source revisions, the Horizon/Mixins revisions and the world seed.
Keep cheats off; do not use Creative, `/give`, `/setblock`, prefilled tanks or
imported inventories. Gather wood, gravel for flint, clay, sand, stone, coal,
copper, iron and gold through ordinary play. Vanilla tools, furnace smelting and
bucket crafting are allowed: this check does not require every tool to be a
TConstruct tool. Keep receipts/screenshots at the checkpoints below.

1. Craft a vanilla crafting table, tools and furnace. Obtain flint from gravel.
2. Craft six Patterns from two planks and two sticks, arranged `ps / sp`.
   Make a second batch so there are enough for both stations and parts.
3. Craft a Part Builder with `pp / ww` and a Tinker Station with
   `ppp / w w / w w`, where `p` is Pattern and `w` is planks.
4. At the Part Builder, use Patterns to make a Pick Head from two flint,
   a Tool Handle from one wood material unit, and a Tool Binding from one wood
   material unit. One ordinary plank supplies one wood unit; an oak log supplies
   four units with plank leftovers. Assemble the three parts as a Pickaxe at the
   Tinker Station.
5. Mine with that pickaxe until damaged. Repair it at the Station with additional
   flint. Verify material consumption, durability recovery and retained parts.
   Save, exit and reopen before proceeding.

Checkpoint: the tool and both stations came from gathered items, and repair did
not duplicate or delete the tool. See [building and repair](playing.md).

## Make the first melting equipment

One clay ball, one sand and one gravel craft two Grout. Smelt Grout in the vanilla
furnace to obtain one Seared Brick per Grout. A convenient budget is 84 bricks
(42 clay balls, 42 sand, 42 gravel), plus furnace fuel. Smelt additional sand into
six glass for the gauge and fuel tank. These are planning quantities, not fixtures.

| Craft | Seared Bricks | Other ingredients | Output |
| --- | ---: | --- | ---: |
| Seared Heater | 8 | None | 1 |
| Seared Fuel Gauge | 4 | 5 glass | 1 |
| Seared Melter | 5 | The **empty** gauge | 1 |
| Seared Faucet | 3 | None | 3 |
| Casting Basin | 7 | None | 1 |
| Casting Table | 7 | None | 1 |
| Seared Fuel Tank | 8 | 1 glass | 1 |
| Seared Drain | 4 | 2 copper ingots | 1 |
| Seared Bricks blocks | 28 | None | 7 |

This uses 74 bricks, leaving ten spare. Use the source crafting patterns in the
recipe view. Filling the gauge before making the Melter prevents that recipe;
it must not silently consume stored fluid.

Place the Heater directly beneath the Melter. Attach a Faucet to the Melter,
with the Basin below its outlet. Fuel the Heater with gathered solid fuel and
melt four copper ingots: each supplies 90 mB, for 360 mB total. Put one Seared
Bricks block in the Basin and pour the copper. The block is consumed to make the
Smeltery Controller. The controller is therefore obtainable **before** building
a Smeltery; it is not a circular prerequisite.

Checkpoint: all equipment was crafted, the controller used exactly one bricks
block, and its copper came from mined ore processed through vanilla smelting.
See [Melter operation](melter-casting.md).

## Cast a metal part and build an alloy Anvil

For a Casting Table check, craft four blank sand casts from one additional sand.
At the Part Builder, place a blank sand cast in the pattern slot and select
Pick Head to make its sand mold. Place that mold on the Casting Table. Melt two
more copper ingots and pour 180 mB into the mold to make a copper Pick Head. The sand mold is
single-use. Replace the existing head at the Tinker Station and verify the
resulting tool's materials, use and save/reload. This does not require gold or a
reusable cast. See [casting parts](playing.md).

Build the smallest open-topped Smeltery around one empty interior block:

- One Seared Bricks block beneath the interior.
- Four side blocks: Controller, Fuel Tank, Drain and one Seared Bricks block.
- A Faucet on the Drain, with the Basin beneath its outlet.

Fill the tank using a lava bucket obtained through ordinary gathering. Iron for
the bucket can come from vanilla mining/smelting. A Heater is not the Smeltery's
fuel source. Keep the remaining four Seared Bricks blocks for the Anvil.

Melt copper and gold in balanced batches. One copper ingot plus one gold ingot
makes 180 mB of rose gold; an empty Basin casts a block from 810 mB. Fourteen
copper ingots and fourteen gold ingots supply 2520 mB: three blocks plus 90 mB
left over. With the one-slot structure, alternate metals and cast a block when
enough **alloy** is present. Do not fill the tank entirely with copper and leave
no room for gold. Refill lava as required; the data check does not predict fuel
consumption or elapsed runtime.

Craft the Anvil with three Rose Gold Blocks across the top and four Seared
Bricks blocks in `mmm /  s  / sss`. Place and open it, then save and reopen the
world. The material budget for this entire route includes 22 copper ingots
(two for the Drain, four for the Controller, two for the Pick Head and fourteen
for rose gold), fourteen gold ingots, and additional iron/fuel for vanilla tools
and buckets.

Checkpoint: record alloy amounts, mold consumption, Anvil ingredients and the
remaining items before and after reload. Confirm no item or fluid duplication.
The [Smeltery guide](smeltery.md) describes representative earlier machine tests;
those are separate from this fixture-free acceptance check.

## Lightweight verification and limits

Run from the repository root:

```sh
python3 -m unittest discover -s tconstruct/tests -p 'test_survival*.py' -v
```

The reachability test starts with explicitly listed vanilla resources and
ordinary vanilla products, never a TConstruct item. It follows shipped native
recipes, resolves the required local tags, checks device declarations, and
rejects the circular block/ingot route as a way to obtain the first alloy.
Removing Grout recipes must break the machine chain. Source recipe checks also
verify the controller, part costs and alloy quantities above.

These tests do not simulate mining, quantities consumed by menus, machine heat,
world generation, tool assembly/repair execution or multiplayer. They do not
claim every material, modifier or Nether branch is reachable. For the integration
run, report each checkpoint as passed, failed or not run and attach screenshots
from the tested binary. A failure should identify the exact recipe/menu/harvest
step and inventory before and after; do not replace the missing input with a
command-supplied item and report the route as passed.

## Smooth basalt blast furnace

The pinned TConstruct recipe also permits making a vanilla blast furnace with three smooth basalt beneath a furnace and five iron ingots. Source comparison tests cover the recipe data. Host recipe loading, crafting and recipe-book display, packaging, save/reload and LAN remain unverified.
