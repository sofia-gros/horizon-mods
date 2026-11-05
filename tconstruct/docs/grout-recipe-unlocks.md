# Grout and brick recipe-book progression

Seven source recipe-unlock advancements now connect the existing early
crafting and smelting recipes:

- Clay Ball unlocks the two-Grout recipe; Clay Block unlocks the eight-Grout recipe.
- Grout unlocks Seared Brick smelting and the Puny Smelting book recipe.
- Magma Cream unlocks both the two-Nether-Grout and eight-Nether-Grout recipes.
  The bulk recipe uses a Magma Block to craft, but the source unlock still uses
  Magma Cream. Its criterion must not be inferred from the recipe ingredient.
- Nether Grout unlocks Scorched Brick smelting.

Each inventory criterion is ORed with the matching recipe-unlocked event;
completing it rewards only the corresponding existing native recipe. These
hidden recipe advancements do not replace the visible guide-acquisition goals,
create new crafting requirements, alter quantities or change furnace timings.

Source Tinkers' Construct revision:
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. Original advancement paths under
`src/generated/resources/data/tconstruct/advancements/recipes/`:

- `building_blocks/smeltery/seared/{grout,grout_multiple,seared_brick}.json`
- `building_blocks/smeltery/scorched/{nether_grout,nether_grout_multiple,scorched_brick}.json`
- `misc/common/puny_smelting.json`

Source recipe IDs `tconstruct:smeltery/<family>/<name>` map to existing
`tconstruct:survival_<family>_<name>` recipes; `tconstruct:common/puny_smelting`
maps to `tconstruct:puny_smelting`. The source recipe-unlocked condition key
`recipe` becomes the host's `recipes`. Other conditions, OR requirements and
telemetry settings retain source behavior.

Tests check complete definitions, native recipe targets and source type/yield/
furnace timing, plus packaged declarations and the existing book-acquisition
advancement. Real inventory/recipe events, recipe-book UI, save/reload, LAN,
and a full natural survival progression run remain unverified. Other unlocks,
custom machinery triggers and recipe ingredient/tag parity remain separate work.
