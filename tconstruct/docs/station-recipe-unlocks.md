# Early workstation recipe unlocks

Obtaining a Pattern unlocks the existing Crafting Station, Part Builder,
Tinker Station and Part Chest recipes. Each source inventory criterion is ORed
with the matching recipe-unlocked event and rewards only that native recipe.
These hidden recipe advancements do not replace visible workstation goals,
change crafting inputs, or introduce new workstation operations.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`src/generated/resources/data/tconstruct/advancements/recipes/decorations/tables/`
with `crafting_station.json`, `part_builder.json`, `tinker_station.json`, and
`part_chest.json`. Native recipe IDs omit the source `tables/` prefix and the
host recipe-unlocked condition uses `recipes` instead of `recipe`. All other
source predicates, OR requirements, rewards and telemetry flags are preserved.

Regression tests cover complete advancement definitions, source recipe
patterns/yields, registered targets, existing visible goals and packaged files.
Actual inventory events, recipe-book UI, crafting, persistence and LAN remain
unverified. Real-machine testing is not a v0.5 completion requirement. Other
recipe unlocks, material-tag conditions and custom machine triggers remain
separate work.
