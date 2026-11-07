# Fresh Windows default-feature BuildOnly

This is a source candidate rooted in integration 9e032ef5b013487ea08b997eb08df84825fb09ba. Execution is disabled by `ready=false` until review, exact freeze, and a separate release. No Rustup, download, Cargo, formatter, Windows API, engine, or game execution has occurred for this candidate.

The controller imports the previously tested source-CI owned phase and raw-child-status worker unchanged. The only Job derivative changes the active-process cap and matching PID query buffer together to 64, and adds a 10 GiB group committed-memory cap. It does not create a new process supervisor. A separate clean checkout of the fixed integration head/tree supplies the ordinary launcher and both packages; CI files remain in their own checkout. The existing ordinary launcher is invoked once, without rewriting its plan, guards, apply, feature selection, or compiler command.

## Preparation and identity

Use the runner's existing `rustup.exe`, record/seal its executable and require its reviewed 1.29.2 version. `rustup toolchain install 1.99.1-x86_64-pc-windows-gnu --profile minimal --component rustfmt --no-self-update` uses a fresh, job-local RUSTUP_HOME. No rustup bootstrap, default-host/profile change, self-update, or alternate toolchain is allowed. Use the installed toolchains directory, not proxies. Require the installed five-component inventory and official distribution manifest's exact date/host/gzip hashes. Rustup is the official checksum-validating component installer; the retained installed tree is sealed independently. This does not claim every installed binary was compared against a separately downloaded tarball.

The fixed rustfmt tar.gz is still checked and its executable must byte-match the selected installation. Exact official WinLibs r3 is downloaded under its SHA and extracted to the selected prefix's mingw64 directory, with bounded regular entries, no symlink/reparse/special/path/case collisions. All payload is retained. Then seal the whole selected toolchain. Record actual rustc commit/version and Cargo version before cache preparation; the unchanged ordinary launcher repeats its Rust/Cargo/GCC/formatter admission.

Fresh official checkouts are restricted to Horizon41f822b and Mixins5a64, with clean Git/head/lock checks. Only `cargo fetch --locked --target x86_64-pc-windows-gnu` for those two original manifests may use the network, in the eventual launcher's isolated CARGO_HOME. The fresh normal stage, official apply's lock completion, and client build remain offline/locked. If a dependency is not in that cache, fail and diagnose; never remove offline/locked or permit a new external dependency. Bundle crates use the original official lock allowlist.

## Actual build route

Invoke the exact ordinary `play_windows_latest.py --build-only` once. It packages this checkout's TC and Pluto sources, records whole-package member seals, and calls the mandatory stage: fresh complete plan, all guards, official apply, then default-feature `horizon-client` fast-profile GNU linking. Audio/controllers remain default; `mods` is requested only when the actual manifest declares it. Official apply owns allowed lock completion. The driver retains the actual fresh plan hash and applied lock identity; only local packages actually listed in that accepted plan may be newly present, while all original package identities/checksums remain unchanged.

Only after the ordinary `.linked.json`/executable hash succeeds, run separate bounded static direct-import inspection and `--help` with a fresh explicit home/game directory. Official Options::read returns None for help and run_with returns before resources/window creation; startup still executes allocator/marker/hook initialization. This is a loader/CLI test, not a rendered gameplay test. Direct import inspection does not prove exhaustive transitive DLL resolution. No assets, GUI, save, or gameplay tests are included.

## Finite stopping limits, not predicted consumption

Observed prior formatter run42332291954 had minimum D:235,145,580,544 bytes, C:30,361,784,320 bytes and available physical RAM14,177,046,528 bytes. Fresh admission remeasures rather than trusting those prior values. Every write volume retains the 10 GiB+128 MiB floor plus a 4 GiB stopping reserve; the work volume initially needs an additional64 GiB reserve. Available physical RAM must remain at least1 GiB. The Job's10 GiB committed-memory hard cap is a different measure.

Total preparation deadline20 minutes, single ordinary stage/link phase90 minutes, inspect/help60 seconds each, workflow job120 minutes and main step115 minutes. Build/preparation logs are capped at32 MiB, inspect at1 MiB. Resource polls and bounded final reads are inherited; normal and failed phases terminate only their owned Job and verify it empty. All failures retain false receipt, actual raw/wrapper statuses, phase/log hashes, available RAM/filesystem minima and reached gates. No automatic retry or fallback.

## Source references

- Standard Windows2025 image/installer: actions/runner-images fad015e47059f40cf0786dcb4593384f6a0720d3, images/windows/Windows2025-Readme.md and scripts/build/Install-Rust.ps1 (Rustup1.29.2; GNU target, not a preselected GNU host compiler).
- Rustup exact cc7b1fda1401d6160c679f2ea5dc258ce86da246, profiles/toolchains/environment-variables docs and src/cli/rustup_mode.rs.
- Rust1.99.1 compiler58cf8f9c35b355199727627d07ae34a4e2846d67 build-manifest minimal profile includes rustc/cargo/rust-std/rust-mingw.
- Official Horizon41f launcher/src/rust_toolchain.rs supplies component and WinLibs hashes. Official Mixins5a64 lockfile.rs completes and validates the applied lock.

The 17 pure/mock source tests pass. They do not establish actual Rustup manifest format/layout, WinLibs archive layout, build resource sufficiency, new plan/apply, or link success. Review is pending; source readiness does not mean execution approval.
