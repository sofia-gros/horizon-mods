# Netherite Tier acquisition

Issue #379: restores `tconstruct:tools/netherite_tier`, with the source Tinker
Tool parent and unchanged inventory criterion. A qualifying stack must pass
both the current-world `tconstruct:modifiable/harvest` tag and an exact
`tconstruct:harvest_tier` stat membership check for `minecraft:netherite`.
The outer tool predicate also retains `tconstruct:modifiable` and count gates.
An ingot, an installed upgrade alone, or a tool with a weaker tier is not enough.
Only the advancement icon key changes to native format.

This restores the missing parent between Material Master and Tinker Tool. It
does not change Material Master's 38 requirements or make its missing material
acquisition paths available.

## Effective tier snapshot

The shared `assemble_with_stats` path writes `tic_harvest_tier` inside existing
`minecraft:custom_data`. This is the same computed tier already used for native
mining rules, including supported head materials and gem upgrade/trait floors.
The native `wooden` rule suffix maps back to source `minecraft:wood`; other
supported names are gold, stone, iron, diamond and netherite. Rebuilds overwrite
the snapshot, or remove it when the tool is not a harvest tool or has no tier.
Existing foreign custom data is preserved. This does not alter mining rules,
stats calculations, inventory transactions or item transport formats.

Legacy tools without the snapshot fail closed until rebuilt through the
existing assembly path, for example a supported station repair or part change.
There is no migration or inference from mining-rule strings. This snapshot
reflects the port's already-supported tier calculation, not arbitrary source
stat modules or custom tiers. Runtime metadata is not recomputed by the
advancement matcher; assembly owns its validity.

## Predicate and dependencies

The bounded `stat_in_set` branch only supports the harvest-tier stat and the
six native tier identities. Unknown stats/values, malformed NBT and unsupported
forms fail closed. `mantle:tag` supports only the declared harvest acquisition
tag, using the resolved current-world snapshot rather than stale host tags.
Existing tool conjunction, material and upgrade behavior is retained.

Stacked on #845. Host [#38](https://github.com/sofia-gros/horizon/pull/38) supplies
item custom-data views; [#41](https://github.com/sofia-gros/horizon/pull/41) makes
metadata-only tier changes reach the existing inventory event. Tag reloads alone
do not manufacture inventory changes or revoke previously earned progress.

Source fixtures/hashes, exact goal/ancestry/package checks, predicate cases,
shared world-tag coverage, and assembly snapshot/rebuild tests were added but
not run. Brief static review only; no tests/builds, CI inspection, version
checks or merges. Actual station interactions, migration expectations, progress,
display, persistence and LAN remain unverified.
