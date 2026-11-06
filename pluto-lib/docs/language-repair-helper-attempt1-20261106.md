# Language merge helper14: first attempt failure

Run [42214234395](https://github.com/sofia-gros/horizon-mods/actions/runs/42214234395),
job 127880155376, attempt 1, source `3c105dbbe0853b5c716516957f684de83d705e30`.
This was a helper-only check, not native placement or Game execution.

- Runner16, repair-source13, placement-source12 passed.
- Actual official f4dc lock was seeded and the standalone serde fixture resolved
  offline with the strict package-identity checks.
- The cold locked Cargo `--no-run` phase returned 0 in 7.619 seconds.
- The subsequent compiler-witness parser rejected serde. Its gate did not pass.
  No helper Rust test was run. Generated OUT_DIR had not yet been sealed.
- Final receipt: `passed=false`, `verified=true`. Source/official/fixture-lock
  verification completed, minimum free 90,046,660,608 bytes. This is not a test
  success or a verified individual compiler-source witness.

The complete bounded GitHub job console is preserved beside this document.
Its 23,248 bytes have SHA256
`c7af3489348396dd0fd56ad9acec447594fe47b3bdd9f86246dede95aaaf7ae5`.
The compile phase's raw 62,909 bytes were represented by hash
`22299442ebd83d9b61799de0465f00ee3192a8c30b37330b86a2256424924615`
only, not retained in the console. No offending raw argv has been recovered.

Source review found that the old parser counted every token ending `.rs`,
including Cargo's leading environment assignments. The public
[serde v1.0.229 manifest](https://github.com/serde-rs/serde/blob/v1.0.229/serde/Cargo.toml)
contains `homepage = "https://serde.rs"`. This is a concrete parser defect and
plausible failure cause, not proof of the missing invocation's exact bytes.

The separately prepared correction parses after the actual rustc executable,
excludes leading environment assignments, rejects ambiguous executable tokens,
and retains exact metadata source/--test/witness-count checks. It adds bounded
crate/path/shape diagnostics without dumping environment values. Three new pure
regressions bring the controller count to 19. The helper, fourteen test bodies,
crate scaffold, dependency policy and workflow are unchanged. No retry has been
made by this checkpoint. The first failure remains part of the evidence.

Writer/stamp acceptance, actual build/cache behavior, pack-priority repair,
Game/UI and full installed composition remain outside this result.
