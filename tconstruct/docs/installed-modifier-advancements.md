# Installed-modifier acquisition goals

Issue #379: restores two source goals without changing their requirements:

- `tools/upgrade_slots` requires Writable, Recapitated, Harmonious, Forecast,
  and Gilded on the same modifiable tool. Its single criterion contains all
  five checks; five separate tools do not satisfy it. Parent: Modified Tools.
- `smeltery/abilities` keeps all 57 separate installed-ability criteria and
  requirements. Progress accumulates across tools and inventory events.
  Parent: Tinkers' Anvil.

All source display settings, ancestry, inventory triggers, and requirements
are preserved. Only the icon keys become native `id` fields. No new recipes,
direct grants, or recipe rewards are introduced.

## Bounded modifier predicate

The shared tool helper now handles `tconstruct:has_modifier` with explicit
`check: upgrades`, a `tconstruct:single` modifier ID, and the source default
presence check (no explicit level field). It reads the existing native
`tic_upgrades` compound under `minecraft:custom_data`: a positive integer
amount for that exact modifier qualifies, including native progressive
amounts. Material traits and computed total modifier levels are not consulted.
The same helper preserves the earlier named `has_upgrades` behavior.

The entire upgrade compound is validated. Invalid IDs/UTF-16, noninteger or
negative amounts, missing data, and unsupported predicates fail closed.
Explicit level fields (including null), all/trait checks, modifier tags,
logical modifier predicates, and short IDs are not implemented or silently
weakened. The outer current-world modifiable tag and nonempty-stack guards
remain. A second modifier-registry lookup is not introduced.

## Dependencies and limitations

Stacked on #847. Host [#38](https://github.com/sofia-gros/horizon/pull/38) supplies
custom-data views; [#41](https://github.com/sofia-gros/horizon/pull/41) detects
metadata-only changes through the existing inventory event. Both parent
chains are already present in this acquisition branch.

This restores acquisition criteria, not the behavior or installation recipes
of every named ability. Several abilities have separate implementation work
or remain unavailable in the bounded port. Full natural completion of the
57-ability challenge is not claimed. No slot accounting, recipe gating,
modifier effects, item persistence format, or network code changes are made.

Pinned fixtures/hashes, exact requirement/ancestry/package checks, and focused
helper cases cover exact installed identity, trait exclusion, malformed
schemas, and the same-tool five-upgrade conjunction. Tests were written but
not run. Brief static review only; no tests/builds, CI inspection, version
checks or merges. Live installation, cumulative progress, notifications,
save/reload, and LAN remain unverified.
