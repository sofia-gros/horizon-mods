# Glass inputs for Seared crafting

The four Seared fuel/ingot gauges and tanks, plus the Seared Casting Tank,
accept the default Forge glass family from the pinned source: vanilla clear,
stained and tinted glass; TConstruct Clear Glass, all 16 stained Clear Glass
colors, Clear Tinted Glass and Soul Glass. Previously the native crafting alias
omitted the 18 Clear Glass variants. Seared Glass itself accepts only the
colorless subgroup: vanilla Glass or Clear Glass. Glass panes and Seared or
Scorched glass do not belong to these ingredient tags.

The native `forge:glass` graph merges Forge 47.4.0 defaults with TConstruct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2` additions. The existing
`tconstruct:source_forge_glass` compatibility alias now points to that graph;
recipe IDs, shapes and output counts stay unchanged. Existing source recipes
and glass acquisition remain unchanged.

`test_seared_glass_inputs.py` checks both source layers, all 37 default members,
registered TConstruct inputs, six recipe shapes/counts, colorless restrictions
and packaging. Survival reachability checks still cover the vanilla route.
These are data tests, not actual crafting, datapack reload, save/load or LAN
validation. Existing Forge namespace ownership and unsupported tag removal
limitations remain; arbitrary third-party pack compatibility is not established.
