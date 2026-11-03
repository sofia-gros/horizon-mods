# Integrating the tested #354 candidate into main

This integration combines candidate `ca87c7016efac7454f6b9f31f4119ef703b90ed4`
with main `5bd65c0a`. It keeps the candidate's Horizon `0c7b5b61` / Mixins
`ddd9700b` target; newer-target work remains in #103. It is not a declaration
that the complete v0.5 release has passed every acceptance gate.

## Preserved behavior

- Keep #360's multiplicative stat rules and #101's post-cancellation mining
  boundary, including the unknown-tag guard.
- Retain lanterns, casting tanks, the fluid cannon, gauges, their resources,
  and shared PlutoLib modules. Historical branch deletions must not remove them.
- Keep main's chute callback snapshot recheck, tool-definition alias handling,
  Step Up item restriction, Ductile toughness multiplier, and legacy tag
  condition support.
- Keep the narrowed Snow Boots/Bouncy/tint holds. The Overwield LAN guest issue
  is tracked in [#363](https://github.com/sofia-gros/horizon-mods/issues/363).

## Verification

The product-source delta from the tested candidate is limited to eight Rust
files: retained main fixes, a compatibility helper, and regression tests.
The earlier official apply/build and representative game results belong to the
candidate, not a newly executed full build of this merge.

Local checks for this integration:

- 47 Rust tests passed against the actual recipe, lookup, tag and stat source,
  plus the extracted legacy-condition implementation and its regression test.
  The bucket-name helper was extracted unchanged from the product file.
- Five PlutoLib inventory API tests passed.
- Four manifest/device/cancellation/chute integration guards passed.
- Rust syntax parsing passed for 468 source files.
- The module-header check inspected 227 references in 22 manifests.
- The existing Python content suite has the same 54 failure/error signatures
  on the baseline and integration (90 tests, one failure, 53 errors, one skip).
  These include source resource assumptions and missing generated assets. This
  suite is not reported as passing. Six source book appearance JSON files also
  fail strict JSON parsing identically on both trees.

No new Windows, full-host build, save migration or LAN run was performed for
this merge. Existing documented tests and known limitations remain scoped to
their stated inputs and revisions.
