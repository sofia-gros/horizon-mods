# Windows native build: minimum remaining gates

This is a source-only execution plan, not a new controller or a successful build. It follows the source-only Windows success at `5e14cf74e14209f312281b582ee44b7cd97406b0`, run [42233831944](https://github.com/sofia-gros/horizon-mods/actions/runs/42233831944). That run parsed PowerShell and passed 20 + 12 + 11 pure tests on Windows Server 2025. It did not execute Cargo, the actual packagers, the game, or a Windows 11 playtest.

## Admit the exact integrated source first

Keep official Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf` and Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`, their checked locks, and an explicitly frozen integrated MOD tree. Generate both complete native bundles using that tree's normal packagers and retain the full individual/combined member seals. An older external Pluto bundle, matching mod id, or matching revision alone is insufficient.

The first complete-plan run for integration `9cc1a01632913f01df1f0bcbb0b74b8bb9e80525` accepted the plan but rejected 10 of 12 existing guards. No apply or type check followed. The terrain metadata candidate `e5c5e5e447886aa29ac2082e15468fc51e5529ab` changes eight metadata leaves, not checker or runtime logic; its selected-witness and source tests do not establish full-plan admission. All mandatory guards must pass the actual complete plan before the existing `stage.py` may apply. Do not use the official launcher's direct-apply route as a fallback around these guards.

## Keep the selected Windows ABI and gameplay features

Use the official launcher's pinned Rust/Cargo **1.99.1**, target **x86_64-pc-windows-gnu**, and hash-pinned **WinLibs GCC 16.2.0 / MinGW-w64 14.0.0 MSVCRT r3** bundle. This is distinct from the official release workflow's Windows MSVC/release configuration. The source declarations do not demonstrate that downloads or this build have succeeded.

The existing launch controller provides explicit CARGO/RUSTC/RUSTDOC and native linker/tool paths, process-local Git configuration isolation, source/toolchain seals, isolated Cargo home/targets/TMP, and `--locked --offline`. An authorized preparation phase may fetch the exact locked dependencies into that isolated Cargo home using the selected toolchain. No fetch/install was performed for this note. Download provenance must come from the official pinned archive hashes, not version text or a locally computed tree fingerprint alone.

Build the actual `horizon-client` binary with the **fast** profile. Preserve default **audio + controllers**, rather than building the no-default-features headless engine. Exact 41f does not declare a `mods` feature, so do not request a nonexistent feature. The existing controller only adds `horizon-client/mods` if the admitted manifest actually declares it; the applied manifest must match the admitted feature contract.

## Link-only validation can precede game resources

The exact official client build script reads repository-owned icon/native API sources, not a prepared Minecraft resource folder. The official workflow compiles and checks executable imports/`--help` before later packaging and resource-dependent smoke steps. Therefore prepared game assets need not block a separate compile/link gate.

The current `play-windows.ps1` entry intentionally performs the full asset preparation and startup sequence, and requires resources at entry. It is **not** a build-only switch. A later bounded build gate should reuse the existing guarded staging sequence and the controller's reviewed `compile_args` contract, stop after binary validation, and not write a game-ready receipt or launch a world. This note does not implement or execute that gate.

Minimum retained evidence: fixed source/bundle/lock/toolchain identities; complete guard results before apply; applied source and generated-input seals; actual compiler/linker argv and target/profile/features; process exit status; resulting binary SHA and x64 PE machine; checked import dependencies; a separately bounded `--help` invocation. A successful `--help` is loader/entry evidence only, not graphics, resources, networking or gameplay evidence.

## Capacity and process conditions remain explicit

Use one Cargo job initially and separate output directories. Monitor every writable filesystem, including target, isolated Cargo home and temporary directories, maintaining the existing 10 GiB + 128 MiB floor. Use a reviewed timeout and cleanup for the owned compiler/linker process tree; preserve partial logs and outputs on failure. No machine-wide setting or privilege change is implied.

Actual cold GNU build disk growth, RAM use and duration are **not measured**. The source-test controller's two-process limit and 40-second phase limit are unsuitable for a Cargo/rustc/linker chain and must not be silently reused or relaxed. The official MSVC job's 40-minute limit is not a GNU timing measurement. A build release needs its own concrete runner capacity and owned-process limits, without weakening the complete-plan guards. No new build controller, workflow, trigger, dependency installation or native process was created for this note.

Only after link validation should the full launch route use an explicitly selected, properly prepared official resource folder, create its separate latest playtest outputs/saves, recheck input/output identities, and attempt startup. Existing normal/legacy worlds remain separate. Actual Windows 11 startup, rendering/GPU output and playability are still unverified.

## Exact source references

- [Official client features](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/crates/horizon-client/Cargo.toml)
- [Official client build script](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/crates/horizon-client/build.rs)
- [Official build workflow](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/.github/workflows/game-build.yml)
- [Official launcher toolchain](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/launcher/src/rust_toolchain.rs)
- [Official native-mod build path](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/launcher/src/native_mods.rs)
- [Reviewed latest launch controller](https://github.com/sofia-gros/horizon-mods/blob/5e14cf74e14209f312281b582ee44b7cd97406b0/tconstruct/tools/play_windows_latest.py)
- [Windows source-check receipt](https://github.com/sofia-gros/horizon-mods/blob/c40c8e858e6da6124b0e57645875a10c83a32226/tconstruct/docs/windows-source-ci-success-20261106.json)
