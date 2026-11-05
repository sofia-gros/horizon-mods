# Seared machinery recipe-book progression

Eleven existing native crafting recipes now have the source recipe-unlock
advancements: Melter, Heater, Casting Basin, Casting Table, Faucet, Fuel Tank,
Fuel Gauge, Ingot Tank, Ingot Gauge, Drain and Chute.

For each recipe, acquiring a Seared Brick satisfies the inventory criterion.
The alternative recipe-unlocked criterion records a recipe learned through
another supported route. The source requirements are OR, not AND, and the
advancement rewards only its corresponding existing recipe. No display/toast
advancement is added and no crafting recipe or machine operation is changed.

The source recipe ID `tconstruct:smeltery/seared/<name>` maps to the existing
native `tconstruct:survival_seared_<name>`. The source `recipe_unlocked`
predicate field `recipe` is adapted to Horizon's `recipes` field. All other
conditions, requirements, rewards and telemetry settings retain the source
semantics from Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`.
The source advancement paths are:

- `recipes/decorations/smeltery/seared/{melter,heater,basin,table}.json`
- `recipes/redstone/smeltery/seared/{faucet,fuel_tank,fuel_gauge,ingot_tank,ingot_gauge,drain,chute}.json`

The Duct is intentionally excluded: its source criterion uses the
`forge:ingots/gold` tag, not Seared Brick. Copying that legacy predicate without
live tag compatibility would wrongly grant it. Other missing recipe unlocks
and the custom Smeltery-open advancement remain separate work.

Tests verify the complete adapted definitions, the rewarded recipes' source
shapes and outputs, and packaged declarations/resources. Real inventory and
recipe events, recipe-book UI, save/reload and LAN remain unverified. This does
not establish a complete survival acquisition run or all original progression.
