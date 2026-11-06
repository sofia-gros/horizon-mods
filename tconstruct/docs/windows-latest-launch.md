# Latest guarded Windows launch candidate

Source-only follow-up to `9fc10b9a4872643ff52b30502c28a95aa4613323`, based on integration `e128d61478971e247b1e3e2e09516cf99652f9fc`. The PowerShell entry is wired to a Python controller; it is not merely an admission design. No PowerShell interpreter, Windows host, Cargo build, source fetch, toolchain installation or game startup was available/run in this validation environment. Twenty latest pure Python/source checks and eleven synthetic package checks passed; twelve historical cache checks also passed against the exact reconstructed 60e42 entry, not against the replaced entry as if behavior were unchanged. This is not a playable-release claim.

## Fixed client contract

- Official Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf` and Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`, exact official HTTPS origins, clean committed source and exact Cargo.lock hashes.
- Official launcher native-client Windows x64 contract: Rust/Cargo 1.99.1, `x86_64-pc-windows-gnu`, WinLibs GCC 16.2.0/MinGW-w64 14.0.0 MSVCRT r3, fast profile. This is deliberately not the separate MSVC release-CI contract.
- Default gameplay audio/controllers are retained. The official launcher only adds `horizon-client/mods` if that feature exists. Exact41f does not declare it, so this cutoff does not request a nonexistent feature. The applied manifest must preserve the admitted default/optional-mods feature contract.
- Only the repository's reviewed `stage.py` may stage. Its complete plan must pass `pluto-lib/tools/check_native_plan.py` before apply. No official-launcher direct-apply fallback, unconditional acceptance, assumed latest nodes or relaxed guards.
- `stage.py` has exactly two additive/compatible edits: an optional `--cli-target-dir`, and use of that explicit isolated directory instead of the old default. Exact inverse reconstruction and wholefile hashes are tested. Without the option, prior behavior is unchanged.

This follow-up also changes only the two revision values in compatibility.json to41f/5a64; the official repository URLs and all other fields are unchanged, with an exact inverse test. That pin delta is not guard, type or game compatibility proof. The controller still requires actual reviewed complete-plan guards; old incompatible guard metadata must reject before apply. It never updates guard identities automatically.

## Explicit preparation, without an automatic installer

`play-windows.ps1` accepts `-HorizonSource`, `-MixinsSource`, `-Resources`, `-Toolchain`, optional `-PlaytestHome`, and an optional compatibility `-PlutoBundle`. Pluto is normally generated from this same checkout, not selected externally. Other missing input folders are requested explicitly; no drive, adjacent checkout, previous launcher setting or wildcard resource discovery is performed. Choose only your official prepared Horizon26.3 resource folder, not a source checkout, modded resource folder or saves directory. No discarded fork path is accessed.

On an approved Windows machine/CI, preparation can use the normal official sources:

- `https://github.com/horizon-rust/horizon` at the exact Horizon commit above.
- `https://github.com/horizon-rust/horizon-mixins` at the exact Mixins commit above.
- The official Horizon launcher's prepared `rust/1.99.1-x86_64-pc-windows-gnu` toolchain, including its `bin` and `mingw64` trees. Its Rust component archive hashes and WinLibs archive SHA256 are pinned in official `launcher/src/rust_toolchain.rs`. Do not substitute a rustup shim, MSVC toolchain or arbitrary same-named binaries.
- User-owned game assets prepared through the official launcher. Toolchain preparation/downloads and licensed game assets are separate prerequisites; this change does not provide them or change their permissions.

This source candidate defaults to offline Cargo, not a permanent ban on official network preparation. Before use, an authorized preparation step can prefetch both exact official workspaces with their locked dependencies into `PlaytestHome/cargo-home-latest-gnu`, using the selected Cargo/Rust/toolchain environment. The minimal commands are `cargo fetch --locked --manifest-path <official-horizon>/Cargo.toml` and the same command for `<official-mixins>/Cargo.toml`, with that `CARGO_HOME`. Execute preparation from a location without inherited Cargo config. Missing crates fail visibly offline; the launcher does not silently fetch, install or retry. A future approved Windows CI may perform those official fetch steps explicitly before invoking this route.

No preparation commands above were executed for this checkpoint. We do not claim the toolchain's whole-tree fingerprint authenticates a download. Use the official archive hashes/preparation route; version checks plus a fingerprint distinguish the actual selected inputs, not their provenance by themselves.

## Inputs, cache and isolation

The generated cache identity contains both official source trees/commits and locks, actual TConstruct inputs, both complete same-checkout native packages, generated member seals and reviewed guard/native/helper files, full selected resource contents, actual selected toolchain contents and reported versions, target/profile/default/optional features, and exact guarded-stage bytes. It is independent of mtime. Input trees reject links/reparse/special entries and ordinary concurrent modification; they are assumed quiescent, not protected against malicious filesystem races. File read budgets are 100,000 files/64GiB total per tree/2GiB per file. Pluto retains its separate 60e42 bounds. Large genuine inputs may reject rather than silently omit files.

All compiler executables are selected from the explicit toolchain. PATH and RUSTC propagate through Python into stage.py's own Mixins Cargo build; Cargo/rustc wrappers and inherited build/target/profile flags are scrubbed. Isolated Cargo home, compiler outputs and temporary paths are under PlaytestHome. Parent Cargo configs and isolated-home config files are rejected. Official workspace config is retained explicitly. Source checkouts are not edited; stage.py exports committed Git trees. The asset and toolchain directories are not modified.

Builds use `builds-latest-gnu/<identity>`, separate from the legacy cache. A ready receipt is written only after build/resource creation and a complete input recheck. Before a cache hit/startup, executable/resource output hashes and all inputs are rechecked. PE machine must be x64. There is no automatic deletion or retry of incomplete outputs.

Worlds use `game-latest-41f822b65978` below PlaytestHome and world name `TConstruct-Playtest`. Existing legacy `game/saves` and normal Horizon worlds are not selected, copied, upgraded or removed. The latest playtest can of course write its own newly selected playtest world while running.

## Remaining execution gates

Actual latest complete-plan/guard admission, Windows GNU build and DLL/runtime checks, actual resource preparation including link behavior (official Windows mod_content uses hard links/copies, which regular-file seals admit; Unix symlinks are not the Windows contract), PowerShell syntax/runtime validation, startup and GPU/playability checks are still required. Source tests do not simulate the native game. Operational free-space/time/process-tree bounds for the authorized Windows runner must be reviewed before a remote build is launched; this controller is not a hard disk or group-memory limit. The current task's local disk floor remains in force, and no local native work was started.

## Review follow-up

The selected absolute Cargo executable is now set in CARGO as well as PATH, because Mixins Index::load reads CARGO directly. All inherited GIT_* redirects are removed; process-local global/system configs and external attributes are disabled, replacement objects are disabled, and local info/attributes rejects. This neither edits Git configuration nor changes permissions. The complete selected toolchain tree is link/reparse-checked before the first version subprocess and checked again afterwards. Error wording now distinguishes preserved normal/legacy saves from writes an already started isolated playtest may have made.

One intermediate pure run stopped on a missing newly planned inverse JSON fixture (17 successful tests, one FileNotFoundError); after writing that fixture, the full18 passed, then20 passed after adding exact-source/attribute regressions. No runtime failure was hidden and no Windows process ran.

## Same-checkout complete native packages

The controller now runs both existing packagers verbatim, rather than installing a raw component directory:

- `python tconstruct/tools/package_hnmod.py --output <fresh>/tconstruct.hnmod`
- `python pluto-lib/tools/package_native.py <fresh>/plutolib.hnmod`

Use a real Git checkout of the combined integration branch: TConstruct's packager uses Git's tracked-file inventory. A ZIP snapshot without that inventory is not silently treated as a complete package. The controller does not create a repository, fetch a fork, rewrite component/header revisions or invent missing files. Inconsistent latest header/components must fail in the normal packager until the integration owner corrects the source declarations.

Each resulting native archive is independently limited to512 regular mode0644 unique members and64MiB unpacked data, with an additional64MiB compressed read budget. Windows-unsafe paths, traversal, links, encrypted entries and case collisions reject. TConstruct's support ZIP is validated separately at10,000 members/64MiB; its allowed content/assets/third_party/docs/tools and Pluto helper roots remain inert resource-preparation data, not an automatically installed content mod. Header/inner native manifest id, version and Horizon revision must agree.

The receipt records archive SHA/bytes, full member-name/hash maps, headers and nested support facts. Its combined seal strips only `mixins/`, excludes only the two outer headers, and rejects collisions. The combined file count is NOT passed off as a single native archive under the512-entry cap. TConstruct's actual packaged bundle parent is passed to `stage.py --mods`; the complete unified Pluto bundle is passed to `--pluto-bundle`. The old raw22KB component manifest is not equivalent to the generated unified manifest.

The optional external Pluto path is accepted only if every relative member and byte equals the package just generated from this checkout. Matching mod id/revision is insufficient. New callers need no external Pluto folder. Prepared archives and both extracted member maps are rechecked at every input snapshot, including ready/startup boundaries. Actual Mod/package sources are sealed before/after packaging and remain fixed through build/startup. A changed source or generated member fails rather than changing the receipt in place.

A fresh temporary packaging attempt is cleaned after completion/failure; only files created for that attempt are removed. Content-addressed complete package outputs are retained, and failed build folders, user source, assets and worlds are not removed automatically. Eleven synthetic tests exercise budgets, Windows/path/mode rejection, headers, nested support, combined collisions, exact external matching, repeat preparation and TC/archive drift. They stub the two packager processes. No actual integration package was generated or native planner/build run for these tests; the actual packager/complete-plan receipt remains the separately coordinated gate.

## Strict pure-output follow-up

The intentional duplicate ZIP negative case now captures and checks exactly one `UserWarning` with `Duplicate name: 'a'`; case-only collisions require no warning. Unexpected warnings are not globally suppressed. All actual verbose, merged stdout/stderr outputs from the20 latest,12 historical and11 bundle suites pass the planned strict named-output validator. Seven controller/parser regressions passed on Linux; PowerShell parsing and Windows Job Object execution are still unrun. No launcher or bundle runtime code changed in this follow-up.

The integration review found one extra terminal LF in the earlier local stage baseline reconstruction. The candidate now removes that single extra byte, and its two-edit inverse returns the actual e128/bff3 Git source,7058 bytes/SHA256 `61f70d88077297d6d827c6d7866cabb7b4983101386ec7233c766e3e36e3d200`. The launch STAGE_SHA and inverse receipt follow this correction; planner/guard/apply ordering and behavior are unchanged.

## First actual Windows source check

Run42230840423 parsed the PowerShell entry successfully but stopped after19/20 latest tests: the remaining test implicitly decoded UTF-8 source with Windows CP1252. The raw stage/source seals matched; the reproduced mojibake SHA exactly matches the failure. The test now hashes bytes and decodes UTF-8 explicitly, with the exact reproduction retained. Runtime source, stage, inverse expected hash and guards are unchanged. Historical12/bundle11 were not reached in that run; see `windows-source-ci-first-result-20261106.json` for the false receipt. A corrected Windows retry is still pending.

The limited three-suite audit found eight remaining implicit text reads, all referring to currently ASCII-only source/JSON, and no read_text call in the bundle suite. Those eight reads now also state UTF-8 explicitly; an AST regression in the existing test requires that policy across all three suites. This changes tests only, not runtime decoding or environment configuration.
