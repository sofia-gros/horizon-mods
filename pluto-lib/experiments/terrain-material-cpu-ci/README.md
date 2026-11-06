# WIP: fixed material CPU proof job

Source preparation only. Do not trigger before independent review and the remote
serial slot release. Production remains exact `43f17427` / archive `cd6de18c`.
This job extends the excluded `7d3b6d04` fixture with one additional secondary-
material timing sample; that changed fixture is regenerated at the new CI head.

The exact dedicated push branch uses public `ubuntu-24.04`, contents-read only,
no cache/artifact uploads, no secrets, no permissions/billing changes and no main
or PR modification. Existing manual CPU/Game and Font jobs are unchanged.
Official Horizon624c/Mixinse8 and the exact official lock are mandatory. All
setup and outputs use the prior successful filesystem/PGID guard, jobs1,
debug0/incremental0, a 10GiB+128MiB floor and extra2GiB before setup. This is a
GitHub-hosted runner route, not permission to use the memory-limited local host.

Required gates: fifteen new and twenty inherited runner checks;23 production
source checks;12 excluded-fixture checks; exact official export; exact ordinary
archive; prepare; complete plan; all12 ordinary guards; additional one-module
guard; official apply; actual host lib/bin type checks; actual renderer unit-test
compilation; two existing material tests; and the one exact CPU test in each of
two independent processes. There is no reuse of an earlier native test result.

Cargo's JSON artifact must identify the real renderer library's test executable
under the external target. Its profile must be test/opt0/debug-assertions on.
After compilation the executable is hashed and invoked directly, never via Cargo:
there can be no new compilation during the two measured executions. Both modes
use `--exact --show-output --test-threads=1`, so successful stdout remains visible
without libtest's `--nocapture` interleaving. Both require exactly one named
passing test,0failed/ignored/measured, the matching mode-complete marker and every
expected case/timing record. Complete stage, lock, generated OUT_DIR, binary,
resource/overlay and checked-out source seals are checked before/after execution.
Missing/failed phases retain a partial false receipt.

Raw compiler/test logs are verified in the runner but **not uploaded**. Console
records include phase SHA/byte length/time, required actual compiler identities
and full-invocation hashes, exact test rows, case/replay/timing JSON, final seals
and the final receipt. Failure adds a bounded tail. A hash is not access to the
unpublished raw log; reports must preserve this evidence limit. This bounds log
volume without making raw-log publication a new permission or artifact action.

Timing is unoptimized original-synthetic CPU mesh timing, not a production
threshold or an optimized-game performance claim. GPU rendering, real resource
packs, gameplay, fluid simulation and TConstruct registration/activation remain
outside this job. A complete test success cannot close those separate tasks.

The new runner additionally seals every source file and symlink target without
following links, including ignored files, rejects ignored checkout additions,
and validates constant official URL/revision/lock and production identifiers
before any fetch. The imported successful runner remains byte-identical.
