# Slimewood recipe-book unlocks

Forty existing Greenheart, Skyroot, Bloodshroom and Enderbark recipes now have
their source recipe-book unlock advancements. For each family:

- Its log unlocks the log-to-wood conversion; its stripped log unlocks the
  stripped-log-to-wood conversion.
- Its planks unlock the slab, stairs, fence, fence gate, door, trapdoor,
  button and pressure plate recipes.

The source inventory criterion OR recipe-unlocked criterion is preserved.
Rewards and the host's `recipes` predicate refer to the existing native IDs.
These are hidden recipe-book records, not new visible progression goals. No
recipes, ingredients, quantities, block behavior or natural supply are changed.

The four plank recipe unlocks remain excluded: their source predicates require
live log-tag matching, which must not be replaced with a fixed member list or
an ignored tag condition. Unregistered signs and hanging signs are also excluded.

Source fixtures preserve the original JSON and hashes from Tinkers' Construct
[`f4e77d43`](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2)
in `tests/fixtures/slimewood_recipe_unlocks`. Focused source, recipe endpoint and
package regressions are included but were not run in this implementation pass.
Actual recipe-book display, inventory events, crafting, save/reload and LAN
remain unverified. This is a bounded part of #379's high-priority unlock work.
