# Grouped PlutoLib foundation validation (latest run 2026-11-06)

Complete library cutoff: `31cf76672d3a7b982aebab86a91a78440afd163e`. Official Horizon:
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`. Mixins: `e8cd0aa38cffef49cb2e088ef716baa107148c02`.

**2 crates, 62 modules, 33 injections, 3 owned states.** This batch adds explicit
immersion-height calculation, immutable curative-policy values, and borrowed
plain-cell count mechanics. Three existing TConstruct helper wrappers delegate
to the latter; their caller layout/admission/notification order stays unchanged.
The full consuming bundle and isolated guide lifetime experiment are not installed.

## Actual complete-package results

- Official plan passed in 23.86 seconds.
- All eleven required source/composition guard groups passed.
- Official apply passed in 24.63 seconds.
- Host lib/bin type-check passed in 1m 12s, offline/locked, debug=0,
  incremental=0 and two jobs.
- 193 Python checks passed, including both complete actual API suites:
  43 gameplay/fluid API and 115 inventory/guide API tests.
- Minimum free space: 10,905,317,376 bytes; disk guard did not stop.
- All 5,045 generated files are sealed. 5,038 byte-identical previous
  files received recorded timestamps after archive verification, with no content edits.

Archive SHA256: `20aa2a95fd066b9714bd6068ad7f654391a32b52d4a2e42aa3b791f5720bbce5`.
Exact artifacts are in `grouped-v13`. Earlier cutoffs and their separate results
remain in the JSON and version history; current source does not retroactively
validate old PR heads.

## Supporting tests and deliberate differences

The source branch passed 36 actual API tests and nine actual World/View tests
using a tiny original bundle accepted by official loaders. The central suite
also includes later fluid registry lease tests. Height sampling reuses the real
decoder; adjusted bounds/timing remain explicit caller responsibilities.
Curative values distinguish unresolved defaults from explicit empty values and
preserve copy versus ordinary-update semantics, without native attachment.

Plain-cell tests passed 31 native cases, including 3,058 valid-input differential
cases through both direct API and actual helper wrappers (6,116 comparisons).
They preserve full-stack clone/move behavior under caller-supplied equality.
Malformed zero counts/limits are deliberately rejected before mutation, not
claimed equivalent to old underflow/zero-stack behavior. The four Game/guest
callers were source-verified unchanged, not executed as live gameplay.

## Remaining limits

This is a **library package type-check**, not a linked game/consumer release.
No full TConstruct bundle, gameplay, save/reload, LAN or ordinary launcher ran.
Library fixtures and modeled endpoints cannot be relabelled as those outcomes.

Plain-cell APIs are not native menu operations: permission, result-slot,
onTake, notification and synchronization hooks remain caller-owned. A merge
keeps the destination identity under the supplied predicate and may discard a
distinct opaque identity. They supply no actor/world/physical-slot authority,
disk transaction or rollback guarantee.

No native EffectInstance curative metadata or same-pass contact hook is installed.
Terrain catalog/geometry is not yet connected to the production mesher. Cloud
insertion is type-checked but unactivated, with four private helper tests and two
factory tests still unrun. Recipe output/remainder authority is separate from
deny-only matching and explicit current-index patching.

Guide request correlation is pure in-process caller-owned state. Native session
ownership, admitted input, reply effects and transport remain unproved and
unactivated. The isolated native lifetime experiment retains its own older
library-plus-experiment proof and is excluded from this package.

Use [guarded staging](native-installation.md#mandatory-development-preflight).
The ordinary launcher does not execute required guards. See the
[migration ledger](shared-api-migration-status.md) for responsibilities.
