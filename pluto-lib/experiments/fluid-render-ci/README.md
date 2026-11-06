# Frozen fluid display library: bounded native validation

Prepared, not executed. Runtime source is local `510f0fa8` / remote `a432ce45`,
based on the ordinary v15 library. The packaged runtime archive must be exactly
`8b9b11366f8f0aa318eebc026723bf12f41276de1a3048651916f692fb40a9e2`.
This experiment does not alter production runtime or mix other WIP groups into it.

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

Source preparation currently has 10 small runner/scope/parser tests passing.
All native gates in this document are unexecuted until an exact matching run
reaches its recorded terminal result.
