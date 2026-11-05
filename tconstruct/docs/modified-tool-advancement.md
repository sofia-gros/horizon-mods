# Modified Tools acquisition criterion

Issue #379: ports `tconstruct:tools/modified` from Tinkers' Construct revision
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. The parent remains
`tconstruct:tools/tinker_tool`; requirements, inventory trigger, custom predicate,
display text, and toast settings are unchanged. Only `display.icon.item` becomes
the native `display.icon.id`. There are no rewards or synthetic grants.

## Host prerequisite

Requires [Horizon PR #38](https://github.com/sofia-gros/horizon/pull/38), which adds
an owned optional `minecraft:custom_data` snapshot to advancement `ItemView`.
The mod reads that field; it does not alter item persistence or invent a parallel
inventory event. Existing inventory fingerprints, criterion selection, progress,
and notifications continue to drive completion. Do not apply this mod change to
a host without that field. Build and runtime integration remain unverified.
Without [host #41](https://github.com/sofia-gros/horizon/pull/41), the host
fingerprint compares item IDs/counts and metadata-only edits wait for a later
inventory change. That follow-on includes custom data in the existing event
gate. Normal acquisition/removal uses the existing inventory event either way.

## Bounded source predicate

The HEAD helper handles exactly `type: tconstruct:tool_stack` with the named
`tconstruct:has_upgrades` predicate. The source requires membership in
`tconstruct:modifiable` and a nonempty installed-upgrades list. The port uses its
existing native saved representation: `minecraft:custom_data.tic_upgrades` is a
compound of namespaced modifier IDs to integer amounts. At least one amount
must be positive, including installed progressive amounts. Material traits and
computed total modifier levels do not count. The helper does not maintain a
second modifier registry; well-formed custom modifier IDs are accepted.

The modifiable tag uses the existing resolved current-world snapshot, including
datapack replacements. It does not use a hardcoded tool list or stale global tag
fallback. The criterion has no vanilla `items` filter, so the native inventory
index keeps it among general candidates and the helper performs the tag check.
Tag reloads do not fabricate inventory events or revoke earned progress.

Empty stacks, absent or malformed custom data, invalid modifier IDs, negative
or noninteger amounts, and empty/all-zero upgrades fail closed. Other source
tool predicate forms also fail closed rather than allowing the native matcher
to ignore the custom `type`. Unrelated native predicate types are untouched.
This is not a general implementation of source slot or compound tool predicates.
The [Material Master follow-on](material-master-advancement.md) adds the bounded
compact material-ID predicate and its goal. The [Slimeskull follow-on](slimeskull-advancement.md)
adds bounded tool conjunctions and item-set membership.

## Review scope

Pinned JSON/Java fixtures and hashes record the source predicate. Focused Rust
checks exercise the production helper; Python checks cover source adaptation,
tag/hook wiring, packaging, translations, and ancestry. Existing world-tag
fixture coverage includes `tconstruct:modifiable`. These checks were added but
not executed. No tests, builds, CI, or version checks were run for this slice.
