# Slimeskull acquisition

Issue #379: restores source `tconstruct:world/slimeskull` with all 16 separate
helmet-material requirements. Each inventory criterion requires both the
`tconstruct:slime_helmet` item and its first saved material matching the named
source identity. An ordinary tool with the same material, or a helmet with the
material in its second slot, does not qualify. Progress accumulates through the
existing advancement inventory events; no direct grant or new recipe is added.

The icon is the native slime helmet. Its source Gunpowder/Enderslime display
NBT is omitted because the host advancement icon supports only an item ID.
This is an icon limitation, not a relaxation of the actual material criteria.

## Composed tool predicates

The existing predicate helper supports nonempty `mantle:and` compositions and
`mantle:set` item-ID membership. The outer source wrapper still requires a
nonempty item in the current world's `tconstruct:modifiable` tag. Every AND
child must pass; unsupported or malformed children fail closed. Item sets
validate every namespaced ID, not just a matching prefix. Existing compact
material/variant/index and named upgrade predicates are reused unchanged.

The adapter bounds nested predicates to eight levels below the root, each
conjunction to 32 children and each item set to 64 entries. Empty conjunctions
and sets, OR/NOT, and unsupported forms fail closed. The Netherite Tier
follow-on adds only the declared harvest item-tag predicate and tier stat;
other item-tag predicates remain unsupported.
These are explicit bounded-adapter limits, not the full source predicate API.

## Dependencies and remaining work

Stacked on #843's material predicate bridge. Host
[#38](https://github.com/sofia-gros/horizon/pull/38) provides custom-data views;
[#41](https://github.com/sofia-gros/horizon/pull/41) also detects metadata-only
part changes. The existing Slimesuit parent is retained; its Sky Island parent
is still missing on this branch, so the full world-tab ancestry is incomplete.

The existing native Slimeskull casting/assembly writes the material list read
here. Named-head casting expansion in #703 remains separate. This slice does
not add head drops, natural entity acquisition, missing materials, helmet
effects, textures, or casting changes, and does not establish that all 16
variants are naturally obtainable.

Pinned fixtures/hashes, exact requirement/package tests, and focused helper
cases cover item-plus-first-material matching, world-tag/count guards,
malformed conjunctions/sets, and depth/width limits. Tests were written but not
run. Brief static review only; no tests/builds, CI inspection, version checks,
or merging. Live acquisition, cumulative progress, UI, persistence and LAN
remain unverified.
