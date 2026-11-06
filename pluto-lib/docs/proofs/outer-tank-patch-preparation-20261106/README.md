# Outer tank proof preparation (not executed)

Runtime/source checkpoint: `794a0c526ca0a96d0ca9c9ff40010611b5968a89`.
All 55 selected source blobs were independently compared with that remote Git
tree. The actual central 28-file inventory API is referenced at its real path,
not copied or substituted. Full official tracked-source and Cargo.lock seals
are checked before and after a future run.

The prepared fixture compiles about 318 KB of imported actual module source,
plus its route/oracle scaffolding and new tests. Static source inspection finds
156 imported test markers and four new named tests (160 expected). This count
is not runtime discovery or a pass. The new cases include 128 Cannon matrix
comparisons, four retained-wire cases, 24 controller comparisons and 576 Drain
route comparisons. Drain uses the existing mock world/Game, not native Game.

The prior 7,779,264-byte fixture executable is only a reference, not an upper
bound: the module set changed, and temporary objects and a newly linked binary
may coexist. A conservative 32 MiB start reserve above the normal 10 GiB +
128 MiB floor is required. This is not a measured maximum. Current headroom is
insufficient, so the run remains blocked; no Cargo metadata, compilation or
native tests were launched. No additional cleanup is authorized by this plan.

The prepared wrapper rejects every compiler invocation except ordinary Cargo
capability queries and one exact new fixture `--test` compilation. Dependencies
and API non-test recompilation are denied before execution. Six pure Python
classification checks pass, but this is not proof that Cargo will find every
dependency Fresh. A rejected invocation or disk stop ends the attempt without
an automatic retry. Continuous 50 ms disk monitoring and bounded process-group
shutdown remain mandatory. Source hashes, actual call logs and test count are
checked before declaring validation passed.

These are workspace-specific, unapplied proof harness files. Restore them in
an authorized fresh `/tmp` proof directory before use, keep the source plan
unchanged, load the existing toolchain environment, and obtain the separate
serial execution grant. The launcher checks an exact-cutoff grant variable and
available capacity before any metadata/build. It must not run in the repository
or overwrite an earlier attempt. The wrapper and runner have Python AST checks
only; the complete harness has not run. No runtime compatibility, actor/slot
authority, game save, notification or full original PR proof is claimed.
