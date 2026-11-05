# Material Master acquisition

Issue #379: restores `tconstruct:tools/material_master` from source revision
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. All 38 named material criteria and
38 separate requirements remain unchanged: progress accumulates by possessing
modifiable tools containing each material, not by collecting raw ingots or by
finding any one material. There is no hardcoded direct grant or recipe reward.
Only the icon key changes from `item` to native `id`.

## Material predicate

The existing source tool predicate hook now supports the compact string-ID
form of `tconstruct:has_material`. It checks actual saved `tic_materials` under
`minecraft:custom_data`, preserving the current-world `tconstruct:modifiable`
tag gate and nonempty item requirement. Unmodified tools can satisfy a material
criterion; upgrades and material traits are not substitutes for material IDs.

An unqualified variant on a namespaced base, such as `tconstruct:wood`, matches
all variants of that base. `tconstruct:wood#oak` is exact; the source's explicit
`#default` predicate requires an unvarianted saved identity. Omitted index or
`-1` scans all slots; nonnegative indices inspect only that position. Invalid
indices, invalid UTF-16/IDs, non-string list entries, and unsupported material
predicate forms fail closed. The complete list is validated even if an earlier
entry matches. Only namespaced identities and nonempty explicit variants are
supported; short IDs, an empty `#` suffix, omitted material wildcards, material
tags, logical compositions, and other tool predicates remain unsupported.

This uses the port's native material identity schema, without a separate
material registry lookup or a second tool-definition validation pass. Existing
tool assembly/part replacement paths are responsible for producing valid
material metadata. The named `has_upgrades` behavior remains unchanged.

## Dependencies and remaining gaps

Stacked on the Modified Tools predicate bridge in mod PR #839. Requires
[host #38](https://github.com/sofia-gros/horizon/pull/38) for custom-data views;
[host #41](https://github.com/sofia-gros/horizon/pull/41) additionally makes
metadata-only part changes reach the inventory event without an ID/count change.

The source parent is still `tconstruct:tools/netherite_tier`, which is not yet
provided by this acquisition branch. It is not replaced with an easier parent;
the visible advancement-tree ancestry remains incomplete. The 38 source
requirements include late-game/compatibility materials. This slice does not
implement missing materials or certify that all 38 are naturally obtainable.
General stat/modifier/slot predicates remain outside this slice. The
[Slimeskull follow-on](slimeskull-advancement.md) adds bounded tool conjunctions
and item-set membership without broadening the material predicate schema.

## Deferred validation

Source fixtures/hashes, exact goal and package checks, and focused production
helper cases cover base/variant/default matching, indexes, malformed metadata,
tag/empty-stack gates, and preserved upgrade behavior. Tests were written but
not run. Brief static review only; no build, CI inspection, version check, or
merge. Live acquisition, cumulative progress, tree display, reload, and LAN
remain unverified.
