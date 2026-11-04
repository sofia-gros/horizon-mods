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

## Steel shard cooking

The pinned source provides smelting and blasting recipes from one Steel Shard to one Steel Nugget, with base times of 50 and 25 ticks respectively and 0.2 experience. Steel Cluster loot contains the shard. Source and registration tests pass; actual acquisition, furnace use, packaging, recipe-book display, save/reload and LAN remain unverified.

## Steel storage conversions

The pinned source's Steel Nugget, Ingot and Block conversions are registered, using additive Forge steel tags. Shaped compression requires the corresponding TConstruct item in the center and eight tagged items around it; unpacking a block or ingot yields nine ingots or nuggets. Source and registration tests pass. The host's existing Forge namespace ownership and `remove` limitations apply. Actual crafting, resource loading, packaging, recipe-book display, save/reload and LAN remain unverified.

## Knightmetal shard cooking

The pinned source smelts or blasts an item in `forge:raw_nuggets/knightmetal` into one Knightmetal Nugget, with base times of 50 or 25 ticks and 0.2 experience. The additive tag includes Knightmetal Shard and an optional Twilight Forest armor shard. Knightmetal Cluster loot contains the native shard. Normal host content preparation filters an unavailable optional item; directly copying the raw tag into an already-prepared root bypasses that filtering. Source and registration checks pass. Existing Forge namespace ownership and native `remove` limitations apply; third-party integration, actual acquisition, cooking, packaging, recipe-book display, save/reload and LAN remain unverified.

## Knightmetal storage conversions

The pinned source's Knightmetal Nugget, Ingot and Block conversions are registered, using additive Forge knightmetal tags. Shaped compression requires the corresponding TConstruct item in the center and eight tagged items around it; unpacking a block or ingot yields nine ingots or nuggets. Source and registration tests pass. Existing Forge namespace ownership and `remove` limitations apply. Actual crafting, resource loading, packaging, recipe-book display, save/reload and LAN remain unverified.

## Seared Reinforcement casting

The pinned source recipe casts one Seared Reinforcement on a Casting Table
from one consumed Pattern and 250 mB of either Seared Stone or Scorched Stone,
with an 89-tick cooling time. The result item and the original item model and
texture are packaged. Its neutral brick template does not add furnace fuel.
The manual Windows resource overlay also registers this item using the neutral
brick template and copies its source model texture.
The ordinary casting catalog now resolves simple arrays of at least two fluid
alternatives when every entry is supported and has the same amount. Unsupported
or different-amount alternatives fail closed; tagged fluid metadata handling
and other casting serializers are unchanged. In the fixed recipe archive this
is the only ordinary casting recipe using a fluid array, though compatible
custom datapacks can also use this subset.

Targeted parser, catalog and casting-state fixtures exercise both fluids,
rejected alternatives, pattern consumption and the cooling boundary. The
hmod and host resource-overlay checks include the result item and original
TConstruct MIT license. Full host compilation, real-world crafting, datapack
reload, modifier application, visual rendering and LAN remain unverified.

To reproduce the Rust fixture, use a local checkout or staged workspace of the
pinned Horizon host with its Cargo dependencies already cached. The runner
requires a clean tracked host checkout at revision `0c7b5b61` and rejects
resolved package versions, sources or checksums absent from that host's lockfile:

```sh
CARGO_HOME=/path/to/cached-cargo HORIZON_HOST=/path/to/horizon \
  tconstruct/tests/run_seared_reinforcement_rust.sh
```

Set `CARGO_HOME` to the Cargo home holding the pinned host's downloaded crates
and registry index. If it is omitted, Cargo uses its normal default. Set
`CARGO_TARGET_DIR` to reuse compiled artifacts. The runner is offline, so a
missing dependency or index entry must be populated separately; it does not
build or test the complete game. Its temporary manifest starts from the pinned
host lock, verifies every resolved version, source and checksum against that
lock, and runs with `--locked`.

## Blazing Bone casting result

Blazing Bone is registered as a regular item for the source casting recipe:
one tagged wither bone is consumed with 200 mB Blazing Blood and 120 cooling
ticks. This does not provide the input or fluid: the stock Necrotic Bone drop remains separate work. Stock piglin barter can
provide limited initial Blazing Blood as described below. A datapack supplying a
valid wither bone and Blazing Blood can use the existing fixed-recipe lookup.
The source luminous item rendering is not reproduced. Host integration,
in-game casting, material traits, saving, and LAN have not been verified.

## Silky Cloth casting result

Silky Cloth is registered as a regular item. Its source table recipe consumes a
cobweb with 90 mB molten rose gold and cools for 52 ticks; its source melting
recipe returns 90 mB at 550 degrees in 52 ticks. This enables those existing
recipe lookup paths when the required resources are supplied. The item's
modifier applications remain separate work. Native
registry IDs may change when items are added. Host integration, real casting
and melting, save/reload, and LAN remain unverified.

## Necrotic Bone item

Necrotic Bone is registered with its pinned-source texture for native and
manual resource paths. The bundled `forge:wither_bones` and
`forge:bones/wither` definitions name this item; actual world tag resolution,
material use and the Blazing Bone casting transaction remain unverified.
The source wither skeleton loot and Severing drops are not implemented by this
change, so ordinary survival acquisition remains unestablished. The item appears in the partial World Creative tab. Host integration, saves and LAN are also
unverified.

Seared and Scorched Lamps can be crafted from a glowstone block surrounded by
four matching bricks, as in the pinned source. The existing lamp blocks and
self-drop loot are unchanged. Source-style recipe-book unlock advancements use
the adapted recipe IDs; runtime unlocking remains unverified. Crafting and placement in a running host, saved
worlds, and LAN remain unverified.

The partial World Creative tab exposes cobalt ore and raw forms, Necrotic Bone,
four source-visible saplings and leaves, four source-visible slime grass
seeds, Earth/Sky/Ichor/Ender Congealed Slime and the three non-vanilla
slimeballs using the source World-tab ID and cobalt ore icon. Existing block and seed
entries in the Blocks tab are retained. Other World items and source ordering
remain incomplete. This is Creative access only, not survival loot. Both native
and manual preparation use paged placement; existing tab IDs are retained.
Actual screen, reload, other-mod combinations and LAN remain unverified.
Manual Creative tab registration does not add these entries to the Search tab;
native Search registration is present but has not been verified in a running
client. Manual and native page positions may differ.

The partial General Creative tab exposes registered Blazing Bone, Silky Cloth
and Seared Reinforcement using the source General ID and Materials and You icon.
Other General entries and source ordering remain incomplete. This provides
Creative access only; it does not establish their survival acquisition or
modifier effects. Manual Search does not include these entries; native Search,
actual display, reload, other-mod combinations and LAN remain unverified.

Four source furnace recipes convert Seared Cobble to Seared Stone, Seared Stone
to Seared Paver, Seared Bricks to Seared Cracked Bricks, and Scorched Stone to
Scorched Road. Each uses one direct item input, 200 ticks and 0.1 experience;
recipe-book unlock definitions use the corresponding adapted native IDs.
Actual furnace execution, recipe-book unlocks, saves and LAN remain unverified.

The source's direct shapeless unpacking recipes yield four matching slimeballs
from one Earth, Sky, Ichor or Ender Congealed Slime block. Earth yields vanilla
slimeballs; the other three yield their named TConstruct slimeballs. The source
recipe-book unlock conditions are adapted to the native recipe IDs. The inverse
four-slimeball recipes use source tags and are separate work. Real crafting,
recipe-book unlocks, save/reload and LAN remain unverified.

Source-shaped 2×2 recipes turn four Seared Stone into four Seared Bricks, or
four Polished Scorched Stone into four Scorched Bricks. The separate existing
four brick-item to one block recipes remain available. Recipe-book unlocks use
adapted native IDs. Actual crafting, unlocks, save/reload and LAN are unverified.

## Blazing Blood via piglin barter

For the exact stock Horizon 26.3 piglin barter table, this port adds the pinned source's
Scorched Lantern entry to the existing weighted pool: weight 20, one to four
lanterns, each holding 50 mB of Blazing Blood. Empty one into a fuel tank to
supply the 1500 °C fuel needed for Netherite alloying. This is one possible
first acquisition route; stock entity melting and Blaze Head melting are not
implemented by this change.

The material-specific Battlesign and tool-parts barter entries (weights 5 and
8) are not implemented, so the total weight and resulting probabilities differ
from the source: this subset selects the lantern with weight 20 out of 489,
versus 20 out of 502 if all three source entries were added to the same host
pool. The host's native loot callback also advances its random
stream once when this entry is selected. A complete normalized table hash
rejects any modified table, including named pools, extra pools and indirect
entry modifiers. A world-pack or native replacement of the barter table is
also rejected, even with identical vanilla bytes. Forge 1.20.1 only fires the
loot-table-load event for built-in tables, not user-pack replacements.

The pinned injector is explicitly bundled at
`data/tconstruct/mantle/loot_injectors/piglin_bartering.json` in the content mod.
Its effective highest-priority resource must match the pinned definition.
Missing, disabled, unreadable, malformed and modified definitions fail closed;
absence never implies the source default. Existing prepared roots must be
regenerated with this content file. A later pack definition takes precedence
over the bundled one, including an empty definition that disables it. Native
injector contributions are unsupported and close the gate. Custom packs may
therefore need their own implementation; this is not general Mantle injector
support.

The `tests/rust/blazing_barter_integration.rs` fixture exercises the staged
host item loot evaluator, lantern fluid transfer and fuel lookup, with stubbed
container capacity and a representative fuel recipe. A minimal content package
containing the injector was read by the host's package/content loaders, written
to its prepared overlay and read back by its built-in DataPack loader. The
complete content package, full client build, actual gameplay, multiplayer and
cross-mod interactions have not been validated.

The pack gate was checked with the pinned source injector fixture and with
extracted, transformed pack-loading code using the host DataPack/DataStore types.
The world loot fixture checks both proof states and cache separation. This is
not a full client build or proof of hook ordering with other native mods.

The stock-table tests require `TCONSTRUCT_TEST_PIGLIN_BARTER_TABLE` to point to
the installation's prepared
`data/minecraft/loot_table/gameplay/piglin_bartering.json`. They fail rather than
skip when that file is absent or differs from the pinned normalized hash;
Minecraft's table is not redistributed as a test fixture.
