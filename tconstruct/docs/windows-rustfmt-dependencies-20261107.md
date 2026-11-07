# Windows rustfmt dependency diagnosis, 2026-11-07

The exact rustfmt used by run [42329829285](https://github.com/sofia-gros/horizon-mods/actions/runs/42329829285) imports three non-system DLLs: `rustc_driver-737f10a507f50706.dll`, `std-f903e3d16df36da5.dll`, and `libgcc_s_seh-1.dll`. Its own pinned archive contains no DLLs. The observed direct child exit was 0xC0000135; this static inspection does not identify which missing dependency the Windows loader encountered first.

One bounded download of the already pinned 4,941,389-byte rustfmt archive was checked against SHA-256 488f4cdc… before safe member reading. The executable is the same 9,723,329-byte SHA-256 537a30b7… file from the run. It was parsed as x64 PE32+ without executing or installing it. Full identities and imports are in the adjacent JSON.

## Official supplier

The archive's compiler commit is `58cf8f9c35b355199727627d07ae34a4e2846d67`. At that exact Rust source:
- [Compiler assembly](https://github.com/rust-lang/rust/blob/58cf8f9c35b355199727627d07ae34a4e2846d67/src/bootstrap/src/core/build_steps/compile.rs#L2333-L2369) copies runtime dynamic libraries into the compiler library directory, which is [bin on Windows](https://github.com/rust-lang/rust/blob/58cf8f9c35b355199727627d07ae34a4e2846d67/src/bootstrap/src/core/builder/mod.rs#L1330-L1360). Windows GNU [keeps std dynamically linked](https://github.com/rust-lang/rust/blob/58cf8f9c35b355199727627d07ae34a4e2846d67/src/bootstrap/src/core/builder/mod.rs#L1147-L1151).
- The [rustc component copies the complete compiler bin](https://github.com/rust-lang/rust/blob/58cf8f9c35b355199727627d07ae34a4e2846d67/src/bootstrap/src/core/build_steps/dist.rs#L524-L530), and [adds libgcc_s_seh-1.dll and libwinpthread-1.dll](https://github.com/rust-lang/rust/blob/58cf8f9c35b355199727627d07ae34a4e2846d67/src/bootstrap/src/core/build_steps/dist.rs#L352-L389). Its conditional flag defaults to true and the inspected builder config does not override it.
- The rust-std component installs target libraries under lib/rustlib; rust-mingw supplies self-contained linker files. Neither is a reason to mix an unrelated DLL release into this formatter.

This identifies rustc as the source-defined supplier. The actual rustc archive and its transitive PE dependency set have not yet been downloaded or verified.

## Minimal next gate

Use the [official Horizon 41f Rust 1.99.1 rustc archive pin](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/launcher/src/rust_toolchain.rs#L110-L129), retaining its bin DLL set and placing the already pinned rustfmt alongside it. Add only this rustc archive to the existing bounded Windows formatter job. Validate member paths, limits, exact hashes and dependency presence before execution, then run version and the unchanged three damage/four terrain regenerations. Record actual filesystem free space and RAM values. Do not obtain Cargo, rust-std, rust-mingw or WinLibs in this gate.

The prior Windows receipt recorded floor checks but no numeric free-space/RAM observations, so it cannot establish heavy BuildOnly capacity. A later build still needs the full official components and WinLibs, a fresh ordinary stage plan, all guards and apply. The existing Mixins apply [completes Cargo.lock](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-cli/src/lockfile.rs); keeping --locked is correct.

No additional archive was acquired during this diagnosis. No new Windows attempt, formatter execution, Cargo, apply, link, game or asset operation occurred.
