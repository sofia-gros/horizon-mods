# Slime crystal recipe-book unlocks

Earth, Sky, Ichor and Ender slime processing now includes the twelve source
recipe-book advancements for the already registered recipes:

- Obtaining a matching slime crystal unlocks its four-crystal block recipe.
- Obtaining matching slime dirt unlocks blasting it into one slime crystal.
- Obtaining a matching slime crystal unlocks blasting it into one slimeball.

Each advancement preserves the source inventory criterion OR its recipe-unlocked
criterion. Rewards point to the existing native recipe IDs. The native predicate
uses `recipes`, as required by the host, instead of the source `recipe` key.
There are no new visible goals, recipes, ingredients or quantities.

The source is Tinkers' Construct
[`f4e77d43`](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2).
Unmodified source fixtures and hashes are in `tests/fixtures/slime_crystal_unlocks`.
Focused source, recipe mapping and package regressions are included but have
not been executed in this implementation pass. Full validation, recipe-book UI,
inventory events, blasting/crafting, save/reload and LAN remain unverified.

This does not add natural slime resources, geode generation or new crystal
growth behavior. It is a bounded part of #379's recipe-unlock work.
