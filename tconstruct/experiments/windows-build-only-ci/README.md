# Fresh Windows default-feature BuildOnly

This source candidate selects ordinary integration 44becb774eeea5742bddad878adada1b4f31b7b1, tree ed2a6c7237a30f7cad36e447e9b71a03b7bffb67, in a separate checkout. `ready=true` records this immutable source choice, not permission to execute. A separate explicit release is required. No new Windows build has occurred for this follow-up; the preceding 89de attempt reached real apply and Cargo before the generated float-range syntax failure.

The controller imports the previously tested source-CI owned phase and raw-child-status worker unchanged. The only Job derivative changes the active-process cap and matching PID query buffer together to 64, and adds a 10 GiB group committed-memory cap. It does not create a new process supervisor. A separate clean checkout of the fixed integration head/tree supplies the ordinary launcher and both packages; CI files remain in their own checkout. The existing ordinary launcher is invoked once, without rewriting its plan, guards, apply, feature selection, or compiler command.

## Preparation and identity

Use the runner's existing `rustup.exe`, record/seal its executable and require its reviewed 1.29.3 version. `rustup toolchain install 1.99.1-x86_64-pc-windows-gnu --profile minimal --component rustfmt --no-self-update` uses a fresh, job-local RUSTUP_HOME. No rustup bootstrap, default-host/profile change, self-update, or alternate toolchain is allowed. Use the installed toolchains directory, not proxies. Require the installed five-component inventory and official distribution manifest's exact date/host/gzip hashes. Rustup is the official checksum-validating component installer; the retained installed tree is sealed independently. This does not claim every installed binary was compared against a separately downloaded tarball.

The fixed rustfmt tar.gz is still checked and its executable must byte-match the selected installation. Exact official WinLibs r3 is downloaded under its SHA and extracted to the selected prefix's mingw64 directory, with bounded regular entries, no symlink/reparse/special/path/case collisions. All payload is retained. Then seal the whole selected toolchain. Record actual rustc commit/version and Cargo version before cache preparation; the unchanged ordinary launcher repeats its Rust/Cargo/GCC/formatter admission.

Fresh official checkouts are restricted to Horizon41f822b and Mixins5a64, with clean Git/head/lock checks. Only `cargo fetch --locked --target x86_64-pc-windows-gnu` for those two original manifests may use the network, in the eventual launcher's isolated CARGO_HOME. The fresh normal stage, official apply's lock completion, and client build remain offline/locked. If a dependency is not in that cache, fail and diagnose; never remove offline/locked or permit a new external dependency. Bundle crates use the original official lock allowlist.

## Actual build route

Invoke the exact ordinary `play_windows_latest.py --build-only` once. It packages this checkout's TC and Pluto sources, records whole-package member seals, and calls the mandatory stage: fresh complete plan, all guards, official apply, then default-feature `horizon-client` fast-profile GNU linking. Audio/controllers remain default; `mods` is requested only when the actual manifest declares it. Official apply owns allowed lock completion. The driver retains the actual fresh plan hash and applied lock identity; only local packages actually listed in that accepted plan may be newly present, while all original package identities/checksums remain unchanged.

Only after the ordinary `.linked.json`/executable hash succeeds, run separate bounded static direct-import inspection and `--help` with a fresh explicit home/game directory. Official Options::read returns None for help and run_with returns before resources/window creation; startup still executes allocator/marker/hook initialization. This is a loader/CLI test, not a rendered gameplay test. Direct import inspection does not prove exhaustive transitive DLL resolution. No assets, GUI, save, or gameplay tests are included.

## Finite stopping limits, not predicted consumption

Observed prior formatter run42332291954 had minimum D:235,145,580,544 bytes, C:30,361,784,320 bytes and available physical RAM14,177,046,528 bytes. Fresh admission remeasures rather than trusting those prior values. Every write volume retains the 10 GiB+128 MiB floor plus a 4 GiB stopping reserve; the work volume initially needs an additional64 GiB reserve. Available physical RAM must remain at least1 GiB. The Job's10 GiB committed-memory hard cap is a different measure.

Total preparation deadline20 minutes, single ordinary stage/link phase90 minutes, inspect/help60 seconds each, workflow job120 minutes and main step115 minutes. Build/preparation logs are capped at32 MiB, inspect at1 MiB. Resource polls and bounded final reads are inherited; normal and failed phases terminate only their owned Job and verify it empty. All failures retain false receipt, actual raw/wrapper statuses, phase/log hashes, available RAM/filesystem minima and reached gates. No automatic retry or fallback.

## Source references

- Actual first-attempt image: windows-2025-vs2026 version20261103.298.1, actions/runner-images tag win25-vs2026/20261103.298, images/windows/Windows2025-VS2026-Readme.md reports Rustup1.29.3, Rust1.99.1 and Rustfmt1.10.0. This image document is evidence for the expected bootstrap version, not a substitute for its actual invocation.
- Rustup1.29.3 exact edbc831cec44dc2858a4a4dce768c74361d027dc: CLI rustup_mode.rs (install/no-self-update/version), src/dist/manifest.rs, src/cli/common.rs and src/toolchain/distributable.rs are byte-identical to reviewed1.29.2 d8c92c6765f85625b20482e165066bf1e98e7297. src/dist/manifest.rs `short_name`/`name` uses reverse_renames for ComponentStatus.name; src/cli/common.rs `list_items` prints that name alone with installed-only. Rust58cf build-manifest/src/versions.rs declares Rustfmt tar name `rustfmt` and preview manifest name `rustfmt-preview`. Thus the exact installed display is `rustfmt-x86_64-pc-windows-gnu`, while manifest package/hash admission remains `rustfmt-preview`.
- Rust1.99.1 compiler58cf8f9c35b355199727627d07ae34a4e2846d67 build-manifest minimal profile includes rustc/cargo/rust-std/rust-mingw.
- Official Horizon41f launcher/src/rust_toolchain.rs supplies component and WinLibs hashes. Official Mixins5a64 lockfile.rs completes and validates the applied lock.

The original 19 pure/mock source tests are retained. They do not establish actual Rustup manifest format/layout, WinLibs archive layout, build resource sufficiency, new plan/apply, or link success. The Job/resource and ordinary/lock reviews found no additional blocker. The bootstrap version/output correction was exercised by the preceding 89de attempt. This does not prove the new generated repair/parser gates or link success.

First attempt42334259441 passed17 Windows pure tests but refused Rustup version before installation. Its158-byte version stream was hashed, not retained, so the exact first line remains unknown. The follow-up retains at most4096 bytes before validation and requires one exact1.29.3 version line plus the two source-known info lines for a fresh RUSTUP_HOME, independent of merged-stream order. Unknown versions/lines, duplicates, non-UTF8 and oversized output still fail. It neither re-runs version nor changes Rust1.99.1, component hashes, WinLibs, Job, budgets, guards or ordinary source.

## Narrow generated boundary repair evidence

This follow-up uses the reviewed ordinary stage repair, not a different planner
or compiler route. After the one build-only phase unwinds, including failure,
the driver checks only the one fresh home/builds-latest-gnu/<64-hex>/stage
receipt named float-range-repair-receipt.json (at most 8 KiB). It preserves the
JSON plus byte/hash identity, and independently checks the staged target's
after hash, single-space inverse, and complete-plan hash. No process is added.
Missing evidence is recorded as absent and cannot make a successful link pass.
Malformed, linked, multiple or drifting inputs are recorded as rejected. The
original build exception is retained rather than overwritten by observation.
Successful linking additionally requires the receipt's stage to be the linked
stage. AST, the complete stage seal and Cargo success are not inferred merely
from a receipt; its whole-stage claims originate in the ordinary helper.

The five repair-observation regressions, three syntax-observation regressions and nineteen existing tests pass (27 total).
The actual generated source and this repair remain untested on Windows.
The immutable ordinary head/tree is fixed in inputs.json;
source readiness does not authorize execution. Job, resource/time/log budgets,
URLs, toolchain, feature selection, existing guards and the one build attempt
remain unchanged.

## Generated syntax receipt

The same failed or successful build attempt also retains the fixed stage
`rust-syntax-receipt.json` (at most 16 KiB). It checks the original parser
scope, edition/stdin/emit settings, exact repaired target bytes and selected
formatter path/hash. Successful linking requires parser exit0, no diagnostic,
and unchanged target/formatter flags. An observed parser failure is retained
without replacing the original build failure. Missing, malformed or mismatched
syntax evidence cannot be promoted to link success. No second build/parser
attempt is introduced by collection.

The ordinary syntax helper was already exercised locally with the official
1.99.1 Linux parser on original-derived good/bad/repaired source: 0/1/0. That
proof is not an actual generated Windows file, Rust type checking or linking.
This follow-up must parse its own freshly generated staged file after repair.

For this fixed head, the complete Git tree contains 508 regular TC native files. The unchanged normal packager adds one header and one nested support entry, giving 510 entries within the existing512 limit. This is source/count verification, not a substitute for actually packaging and checking all bytes in the job.
