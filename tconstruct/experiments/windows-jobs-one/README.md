# Explicit Cargo concurrency for the Windows launcher

Base: integration 5265ce25fac98f690eeb5b0c36df94caf3a85454.

The only ordinary code change sets `CARGO_BUILD_JOBS=1` after clearing inherited build overrides in `build_environment`. The same selected environment already reaches the stage Python process and the later client Cargo build. Stage's Git, CLI Cargo build, planner, guards, and apply subprocesses inherit it. Official Mixins' Cargo metadata/lock completion therefore also inherit this limit; no command or feature selection is changed.

Default audio/controllers, conditional `mods`, target/profile, `--locked`, fresh plan, all guards, and apply ordering remain unchanged. This controls Cargo's parallel jobs only. It does not bound all build-script/compiler descendants and is not a substitute for a separately reviewed Windows Job process/resource budget. The existing formatter's three-process/120-second bounds are not a heavy-build configuration.

Six pure/mock tests cover hostile prior overrides, exact old-environment equivalence except the new key, both real launcher command paths with mocked external work, guard failure, stage's inherited subprocess environment, and exact whole-body inverses. Two historical test adapters reverse this precise new line before applying their prior inverse witnesses. No old hash or negative condition is removed.

No Cargo, compiler, Windows API, download, formatter, engine, or game runs are part of these tests. Windows build success and resource use remain unverified.

Local validation: all 97 selected pure/mock tests passed. Two initial aggregation commands named missing test modules (3, then 1 import errors); corrected actual module names passed without changing assertions. An initial aggregation also ran the existing stage mock/Git fixture, which passed. No real Cargo or native build ran.
