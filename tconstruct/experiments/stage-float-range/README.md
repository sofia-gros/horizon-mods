# Known generated float-range boundary: stage connection

Base ordinary source: `9e032ef5b013487ea08b997eb08df84825fb09ba`.
This candidate connects the narrow `stage_float_range_repair` helper immediately
after successful official apply. The previous complete plan, package contract,
all guard calls, apply arguments, feature selection and locks are unchanged.
No new CLI argument, formatter process, dependency update, or general rewrite is
introduced. The helper accepts only its pinned original sources and target-file
plan closure, then restores one ASCII space in the known generated expression.
Its precise input checks and refusal cases are documented with the helper.

The launcher pins the new complete stage SHA. Its existing `package_sources()`
includes the whole tools directory, so the helper is included in source seals
and the stage change changes the cache identity. Successful repair writes the
fresh `stage/float-range-repair-receipt.json`; it records before/after bytes and
hashes, one inserted byte, exact inverse, and unchanged other staged entries.
This receipt does not claim Rust AST or compilation success.

`inverse.json` records exact whole-file before/after hashes and unique reverse
edits for all six modified existing files. Four test adapters first undo this
new layer and then run their previous inverse/hash assertions unchanged. The
ordinary stage export fixture uses an explicit repair test double because its
fake official revisions cannot satisfy the real helper; it checks that apply
and the complete-plan guard happened before the double, and that guard refusal
never creates a repair receipt. Its existing 12 stage calls and archive contract
remain intact.

## Verification and limits

The new six pure connection tests plus the previous 97 selected pure tests pass
(103 total). After copying the four exact helper/test/proof files, all 127 tests
(103 plus 24 helper tests) pass together. They inspect actual AST statements and execute only the final
apply/repair tail with test doubles. The first sparse-context run had one import
context error because the old UTF-8 test required the damage guard file. Restoring
that exact unchanged 9e032 file made all 103 pass; no assertion was relaxed.
The Git/export stage fixture itself has not been executed here. No new native
compiler, formatter, planner, official apply, Windows build, or game was run.
The actual failed generated start.rs file was not retained, so acceptance of
that complete generated file and the subsequent Cargo syntax gate remain for
the next separately reviewed execution. This source checkpoint is not a build
success claim.
