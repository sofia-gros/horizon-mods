# Frozen fluid display library: bounded native validation

Validation is in progress. Runtime base is local `510f0fa8` / remote `a432ce45`,
based on the ordinary v15 library. The packaged runtime archive must be exactly
`b448763e77488d5f7c119d99d675351a8d3244db1a80b3dfd374fa439f039e81`.
The first run exposed the production tint closure type error described below. Only that minimal runtime correction is added; no other WIP group is mixed in.

The dedicated push workflow is restricted to the public target repository and
`checkpoint/plutolib-fluid-render-ci-20261106`. Start only after the preceding
Guide Font12 run reaches terminal status. It uses the standard Ubuntu 24 runner,
contents-read only, no persisted credentials/cache/artifact action, no PR or
billing changes. Supersession cancels only this dedicated branch's own group.
The old manual CPU/Game and automatic Guide branches remain unchanged.

## Separate gates

1. Nineteen Python source/package guards.
2. Thirty tests compiled from the three exact production std-only modules:
   bindings 11, geometry 7, providers 12. No substitute renderer or owner structs.
3. Twenty-one tests from the complete real dependency-backed Pluto API's fluid
   test module. The standalone lock must contain only official identities plus
   the exact local API identity. The remaining API tests are not claimed here.
4. Build exact official Mixins, package exact runtime, plan, run all twelve
   complete-plan/source guards, then apply to the pristine official source.
5. Actual offline/locked Horizon host library and named binary type checking,
   with observed rustc invocations for API, renderer, client-library adapter, host library and host binary separately. A cargo Fresh or
   zero-work success is insufficient for these designated targets.
6. Compile the complete actual renderer test target and execute exactly the two
   native BakedFace material-attribute tests. They use actual native types, but
   are not asset-loading or meshing execution tests. No optional resource skip.

The fixed source revisions are Horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0
and Mixins e8cd0aa38cffef49cb2e088ef716baa107148c02, both horizon-rust originals.
The repository's portable archive reader produces the pristine export; no
fork, old native patch, copied host model or old d63 CPU oracle is used.

All subprocesses, including apt/rustup, share a continuous 10 GiB + 128 MiB disk
floor over checkout/work/target/Cargo/root/RUSTUP_HOME output filesystems.
Cold setup additionally requires 2 GiB reserve. Jobs 1, debug 0, incremental 0;
TERM 1 second then whole own-group KILL/reap 2 seconds. These are bounded polling
monitors, not a quota. Local native execution remains prohibited.

Exact MOD, official exports, extracted package, API and applied-stage source /
manifest / Cargo.lock seals are compared after execution. Native generated
OUT_DIR files are sealed after test compilation and compared after execution.
Every required test name and passing count is checked; zero/ignored tests fail.
No target directory is inside a sealed source tree. Evidence is printed to
ordinary job logs and summary rather than uploaded to an artifact/cache service.

A partial failure reports completed gates separately. None of these gates
proves 70 actual content families are registered, resource reload behavior,
loader resolution, full-section fallback/cache replay, meshing equivalence,
CPU performance, upward simulation/contact, live gameplay or GPU display.
No TConstruct consumer is activated. Those claims need later dedicated evidence.

Source preparation currently has 12 small runner/scope/parser tests passing.
Completed and unreached native gates are recorded separately below.

## Preserved initial failure

Run 42172857479 at 1b60c96a passed source 19, then std compilation failed with E0277 (`u32 & i32`) in the tint closure. No Rust tests, API, planner, apply or host type gate ran. Minimum free space was 91,144,896,512 bytes. The retry explicitly types the shift, masks and divisor as u32 without changing channel arithmetic, and updates exact bundle/module/source pins. This is a correction to the a432 base, not reused evidence of a pass.

## Preserved second failure

Run 42173321338 at 60054f8b passed source 19 and compiled/executed all 30 std tests (30 passed, zero failed/ignored). The CI exact-name check then rejected five expected names because they incorrectly used the tests module instead of revision_view_tests/appearance_tests. API, plan, guards, apply, host type and native material gates were not reached. Minimum free space was 91,166,392,320 bytes. The next retry corrects only those qualified names and adds a source-module regression; runtime bytes and archive are unchanged. The failed final gate is not presented as a complete run success.

## Preserved third failure

Run 42174045690 at 2b458c72 passed source 19 and std 30. The API command selected only fluid::tests::, so 10 original tests passed but 11 revision/render cases were filtered out. The exact-21 assertion correctly rejected this partial result. The next retry selects fluid::, corrects nested expected names, and checks all three exact source inventories plus filter coverage before setup. No runtime changes are included. Plan/apply/type/native-material remain unreached. Minimum free space was 90,912,833,536 bytes.
