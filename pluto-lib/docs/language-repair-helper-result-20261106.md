# Language merge helper: fourteen tests passed, native integration still open

[Run 42217851276](https://github.com/sofia-gros/horizon-mods/actions/runs/42217851276),
job 127892232905, exact source `1dc4c6dab50c002a77fb1e8874c8576a66dc09ca`.
The reviewed parser-only retry completed successfully. FINAL_RECEIPT is
`passed=true`, `verified=true`, `error=null`. All six required gates completed.

## Verified scope

- Actual frozen `merge.rs` with official f4dc-locked serde 1.0.229 and
  serde_json 1.0.151, plus std. Actual cold compiler witnesses for those two
  dependencies and the exact `--test` fixture matched metadata paths.
- All fourteen exact Rust names passed once, zero ignored and zero filtered.
  The tests cover strict lower-only merge/higher conflict priority, exact
  unchanged higher bytes when no added keys or unsupported input, duplicate
  rejection, UTF-8/placeholders, budgets, scope, and lower read failure.
- Twenty runner checks, thirteen repair-source checks and twelve placement
  synthetic/source checks passed. The last twelve do not imply placement.
- Locked offline graph with only the fixture as a new local identity; complete
  Mod/official source and fixture/metadata/lock seals; generated OUT_DIR inputs
  unchanged across execution. The test command's raw log contained no recompile
  according to the frozen validator. This is post-invocation detection, not a
  structural ban on compiler launch.
- All fifteen command phases exited 0. Cold helper compilation took 5.612 s,
  fourteen-test command 0.201 s. Minimum free space was 90,046,554,112 bytes.

The full bounded console is retained: 28,695 bytes, SHA256
`41dbc464d7959501f0471540bbd0439e1147692ebed71786375c2b27a21703fb`.
Its separate `ACTUAL_HELPER_COMPILERS` and `VALIDATED_TEST_NAMES` records match
the final receipt. The original raw compiler and Rust test logs were validated
inside the frozen runner before console reduction; their hashes/byte counts
are recorded, but their full ephemeral bytes were not retrieved afterward.
This closure does not claim independent parsing of unavailable raw Rust output.
The stored final receipt was parsed and serialized in Python to preserve
integers without JavaScript Number rounding.

## What this does not prove

No Horizon native host, Mixins apply or accepted placement, actual writer/build
cache, selected-pack priority repair, full composition, Game/UI, or runtime
activation was tested here. The earlier stamp placement was rejected and
remains rejected. The separate root-key source proposal was not part of this
run. Helper correctness is one gate, not completed translation repair.

## Preserved first failure

Run 42214234395 at `3c105dbb` stays recorded as failed: cold compile exited 0,
then the former compiler-witness parser rejected serde; no fourteen tests ran.
The concrete parser flaw treated legal Cargo environment values ending `.rs`
as source paths. The missing raw argv means that first failure's precise token
was not recovered. The correction and its verbose-output admission regression
changed no Rust helper, fixture, lock policy or workflow. This new success does
not rewrite the first failure. The successful trigger stays frozen.
