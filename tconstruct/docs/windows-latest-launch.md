# Latest guarded Windows launch candidate

Source-only follow-up to `60e42cfc21f130e79670ffe1c591fc74a1ef23e1`, based on integration `e128d61478971e247b1e3e2e09516cf99652f9fc`. The PowerShell entry is wired to a Python controller; it is not merely an admission design. No PowerShell interpreter, Windows host, Cargo build, source fetch, toolchain installation or game startup was available/run in this validation environment. Sixteen latest pure Python/source checks passed; twelve historical cache checks also passed against the exact reconstructed 60e42 entry, not against the replaced entry as if behavior were unchanged. This is not a playable-release claim.

## Fixed client contract

- Official Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf` and Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`, exact official HTTPS origins, clean committed source and exact Cargo.lock hashes.
- Official launcher native-client Windows x64 contract: Rust/Cargo 1.99.1, `x86_64-pc-windows-gnu`, WinLibs GCC 16.2.0/MinGW-w64 14.0.0 MSVCRT r3, fast profile. This is deliberately not the separate MSVC release-CI contract.
- Default gameplay audio/controllers are retained. The official launcher only adds `horizon-client/mods` if that feature exists. Exact41f does not declare it, so this cutoff does not request a nonexistent feature. The applied manifest must preserve the admitted default/optional-mods feature contract.
- Only the repository's reviewed `stage.py` may stage. Its complete plan must pass `pluto-lib/tools/check_native_plan.py` before apply. No official-launcher direct-apply fallback, unconditional acceptance, assumed latest nodes or relaxed guards.
- `stage.py` has exactly two additive/compatible edits: an optional `--cli-target-dir`, and use of that explicit isolated directory instead of the old default. Exact inverse reconstruction and wholefile hashes are tested. Without the option, prior behavior is unchanged.

The current integration still has older compatibility/guard metadata. This controller intentionally rejects it before compilation until the integration owner lands genuinely reviewed latest manifests/guards after actual plan evidence. Changing this launcher is not a substitute for that work.

## Explicit preparation, without an automatic installer

`play-windows.ps1` accepts `-HorizonSource`, `-MixinsSource`, `-PlutoBundle`, `-Resources`, `-Toolchain`, and optional `-PlaytestHome`. Missing input folders are requested explicitly; no drive, adjacent checkout, previous launcher setting or wildcard resource discovery is performed. Choose only your official prepared Horizon26.3 resource folder, not a source checkout, modded resource folder or saves directory. No discarded fork path is accessed.

On an approved Windows machine/CI, preparation can use the normal official sources:

- `https://github.com/horizon-rust/horizon` at the exact Horizon commit above.
- `https://github.com/horizon-rust/horizon-mixins` at the exact Mixins commit above.
- The official Horizon launcher's prepared `rust/1.99.1-x86_64-pc-windows-gnu` toolchain, including its `bin` and `mingw64` trees. Its Rust component archive hashes and WinLibs archive SHA256 are pinned in official `launcher/src/rust_toolchain.rs`. Do not substitute a rustup shim, MSVC toolchain or arbitrary same-named binaries.
- User-owned game assets prepared through the official launcher. Toolchain preparation/downloads and licensed game assets are separate prerequisites; this change does not provide them or change their permissions.

This source candidate defaults to offline Cargo, not a permanent ban on official network preparation. Before use, an authorized preparation step can prefetch both exact official workspaces with their locked dependencies into `PlaytestHome/cargo-home-latest-gnu`, using the selected Cargo/Rust/toolchain environment. The minimal commands are `cargo fetch --locked --manifest-path <official-horizon>/Cargo.toml` and the same command for `<official-mixins>/Cargo.toml`, with that `CARGO_HOME`. Execute preparation from a location without inherited Cargo config. Missing crates fail visibly offline; the launcher does not silently fetch, install or retry. A future approved Windows CI may perform those official fetch steps explicitly before invoking this route.

No preparation commands above were executed for this checkpoint. We do not claim the toolchain's whole-tree fingerprint authenticates a download. Use the official archive hashes/preparation route; version checks plus a fingerprint distinguish the actual selected inputs, not their provenance by themselves.

## Inputs, cache and isolation

The generated cache identity contains both official source trees/commits and locks, actual TConstruct inputs, selected Pluto bundle plus reviewed guard/native/helper files, full selected resource contents, actual selected toolchain contents and reported versions, target/profile/default/optional features, and exact guarded-stage bytes. It is independent of mtime. Input trees reject links/reparse/special entries and ordinary concurrent modification; they are assumed quiescent, not protected against malicious filesystem races. File read budgets are 100,000 files/64GiB total per tree/2GiB per file. Pluto retains its separate 60e42 bounds. Large genuine inputs may reject rather than silently omit files.

All compiler executables are selected from the explicit toolchain. PATH and RUSTC propagate through Python into stage.py's own Mixins Cargo build; Cargo/rustc wrappers and inherited build/target/profile flags are scrubbed. Isolated Cargo home, compiler outputs and temporary paths are under PlaytestHome. Parent Cargo configs and isolated-home config files are rejected. Official workspace config is retained explicitly. Source checkouts are not edited; stage.py exports committed Git trees. The asset and toolchain directories are not modified.

Builds use `builds-latest-gnu/<identity>`, separate from the legacy cache. A ready receipt is written only after build/resource creation and a complete input recheck. Before a cache hit/startup, executable/resource output hashes and all inputs are rechecked. PE machine must be x64. There is no automatic deletion or retry of incomplete outputs.

Worlds use `game-latest-41f822b65978` below PlaytestHome and world name `TConstruct-Playtest`. Existing legacy `game/saves` and normal Horizon worlds are not selected, copied, upgraded or removed. The latest playtest can of course write its own newly selected playtest world while running.

## Remaining execution gates

Actual latest complete-plan/guard admission, Windows GNU build and DLL/runtime checks, actual resource preparation including link behavior (official Windows mod_content uses hard links/copies, which regular-file seals admit; Unix symlinks are not the Windows contract), PowerShell syntax/runtime validation, startup and GPU/playability checks are still required. Source tests do not simulate the native game. Operational free-space/time/process-tree bounds for the authorized Windows runner must be reviewed before a remote build is launched; this controller is not a hard disk or group-memory limit. The current task's local disk floor remains in force, and no local native work was started.
