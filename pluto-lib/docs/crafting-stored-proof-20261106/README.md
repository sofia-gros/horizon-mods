# Stored crafting facts: API153 and finite16 proof

First attempt **PASS**, exact source
`86617f80fc7d927bfc2971cedacb35783b189ce9`, tree
`407dd3d9b37a7866e33421679c8468e1eae648b3`.

- [Workflow run 42247376045](https://github.com/sofia-gros/horizon-mods/actions/runs/42247376045)
- [Job 127993024798](https://github.com/sofia-gros/horizon-mods/actions/runs/42247376045/job/127993024798)
- [Frozen controller](https://github.com/sofia-gros/horizon-mods/blob/86617f80fc7d927bfc2971cedacb35783b189ce9/pluto-lib/experiments/crafting-stored-ci/run.py)
- [Frozen source/test inventory](https://github.com/sofia-gros/horizon-mods/blob/86617f80fc7d927bfc2971cedacb35783b189ce9/pluto-lib/experiments/crafting-stored-ci/inputs.json)

The dedicated branch was verified absent, created once at this exact commit, and
read back before monitoring. No automatic retry occurred. The branch remains
fixed. Ordinary integration pushes do not start this dedicated workflow.

## Actual results

The source-only setup gates passed 31 controller, 21 preparer and 13 renderer
checks. Cold official dependencies were fetched against Horizon
`41f822b659789780f58d32c63ddfb6887ea1b8bf` and its pinned lock. Subsequent metadata
adaptation and compilation were locked/offline, with one real inventory API path.

Both fresh harnesses compiled before either binary was run. Actual compiler
witnesses identify horizon-core, horizon-client-core, horizon-items, horizon-world,
the API library, API test harness, and finite test harness. Artifact identity,
profile, source path and uniqueness were checked. The two binaries each listed
their entire inventory and ran once without filters:

- API: **153 passed**, including all 147 prior tests and six new stored-reader
  compatibility tests. Failed, ignored, measured and filtered counts are zero.
- Finite native policy fixture: **16 passed**, including the unchanged old-policy
  comparison, native component view and existing tank case. All zero-skip counts
  match the exact names in the frozen inventory.

All 21 process phases exited zero, with no reported phase, raw-read or summary
error. The final report has `passed=true`, `verified=true`, with source, official,
workspace/lock, binaries and generated-output seals retained. The minimum measured
free space was **89,852,682,240 bytes**, above the 10 GiB + 128 MiB floor. The
bounded runner verified its own process groups absent after each phase.

## Evidence and independent console checks

The exact full FINAL_RECEIPT object obtained from the job console is retained
locally for independent review, preserving all fields, named inventories and
compiler witnesses. It is not part of this two-file summary publication.
`verification.json` records the separate console verification: both fixture
receipts and all 21 phase receipts equal the final report; every actual test name
and both zero-skip result summaries match the frozen source; the old/new API
groups remain 147/6; compiler owners and final seal fields are present.

The decoded GitHub console contains 199,973 UTF-8 bytes, SHA-256
`ef3944f961efb06b64acfbae230f5bd5e24eb8b307c26ef749517b928339ee72`.
It was retained locally during review, not included in this two-file repository
closure. Its hash is not a download link. Likewise the phase raw stdout/stderr
files were validated by the CI runner and represented by hashes/lengths and
bounded compiler witnesses; those ephemeral raw streams are not made retrievable
by this documentation. The linked GitHub job is the external console source.

## What this closes, and what it does not

This proves the source-pinned real API and finite native stored-reader/policy
fixture compile and pass at the stated official cutoff. It covers preserving the
old matching API, explicit deny-on-missing stored capability, one registry,
unlocked callbacks, and finite comparisons for material-required crafting and
filled-tank denials, including retained-Wire input handling through borrowed
stored fields.

It does **not** prove Mixins plan/apply, installed module visibility, actual
Recipes constructor registration/bootstrap, full Game typing, the Overslime pure
pair provider, station transaction placement, LAN/gameplay or complete composition
guards. No ordinary hook was removed or activated by this verification. Those
authorities and their existing behavior remain separate gates before adoption.

The prior dedicated-workflow preparation's initial pure-test assertion mismatch
is documented in the frozen controller README. It occurred before this CI and
does not change this run's first-attempt success or conceal a native retry.
