# Slimewood recipe unlock fixtures

Forty original generated advancement JSON files from Tinkers’ Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. File bytes are unchanged; hashes
are in `sha256.json`. Sources are under
`src/generated/resources/data/tconstruct/advancements/recipes/`:

- `building_blocks/world/wood/{wood}/{log_to_wood,stripped_log_to_wood,slab,stairs}.json`
- `redstone/world/wood/{wood}/{button,pressure_plate,door,trapdoor,fence_gate}.json`
- `decorations/world/wood/{wood}/fence.json`

The four woods are Greenheart, Skyroot, Bloodshroom and Enderbark. Native
advancements map rewards and the recipe-unlocked criterion to existing native
recipe IDs and use the host `recipes` predicate key. Source inventory predicates
and OR requirements are preserved. Tag-based planks, signs and hanging signs
are deliberately excluded.

Source: https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2
License: `tconstruct/third_party/tconstruct/LICENSE`.
