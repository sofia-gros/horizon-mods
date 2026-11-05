# Necrotic Bone acquisition

Wither Skeleton entity-table bone drops become Necrotic Bones when the source
`[recipes] witherBoneDrop` setting is true (the default). False, invalid or
unreadable configuration fails closed. Configuration and pack proof are rebuilt
at world table load, including `/reload`; no global mutable cache is introduced.

The pinned TConstruct source at `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`
(`data/tconstruct/loot_modifiers/wither_bone.json`) uses Mantle's `replace_item`:
([Mantle implementation](https://github.com/SlimeKnights/Mantle/blob/3cf1c1628717163fef50ef44ec01847c723b1618/src/main/java/slimeknights/mantle/loot/ReplaceItemLootModifier.java)):
only `minecraft:bone`, only `minecraft:entities/wither_skeleton`, same rolled
count, fresh replacement stack without the old components. The packaged source
modifier is checked canonically. Its original global-modifier list includes
`tconstruct:wither_bone`; the rest of that list is not implemented here.

The bounded port runs at the return of the non-generic `mobs::drops::roll` wrapper, before the host appends
held/worn equipment. It does not consume randomness. A custom native loot
function would consume a context seed and change RNG, so it is not used.
An invalid-resource-location empty table carries proof in the loaded table set;
pack-supplied proof tables are removed before this proof is set. Configuration
and pack-proof transport tables are removed. No real loot resource is replaced.

Unsupported overrides fail closed: enabled datapacks replacing the target loot
table, any Forge/NeoForge global-modifier list or modifier definition, and native
contributions to that table or any modifier. This does not emulate arbitrary
Forge global modifiers. Overrides are detected before parsing, including malformed
files. Disabling vanilla or omitting/changing the packaged source also closes the
gate. Other mob tables, coal/skull drops and held equipment remain unchanged.

Tests cover configuration true/false/invalid and repeated rebuilds, pack/native
conflicts, canonical source definition, actual staged host entity rolls, unchanged
random state, stack counts, component replacement and unrelated tables. The
pack-discovery fixture is a stub, not an in-game reload test. The staged host test
uses synthetic tables, not redistributed Minecraft data. Actual survival play,
dedicated-server/LAN behavior and full live `/reload` remain unverified. Non-entity
roll entrypoints (for example loot-command paths) are outside this bounded port.
