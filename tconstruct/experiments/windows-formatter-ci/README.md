# Windows 1.99 formatter with its matching rustc runtime

Source-only follow-up to 9846. No new archive, Windows attempt, installer or native
execution has occurred for this candidate. The independent static investigation
is recorded in docs/windows-rustfmt-dependencies-20261107. Actual run42329829285
returned direct formatter status0xC0000135 (wrapper4294967295), before version or
any helper regeneration. Its failure is preserved, not rewritten as a success.

## One additional pinned component

The only new network input is the official rustc1.99.1 Windows GNU archive from
2026-10-15, SHA43b17e9a… as fixed in official Horizon41f launcher/rust_toolchain.rs.
No Cargo, rust-std, rust-mingw, WinLibs, rustup or alternative source is obtained.
The existing rustfmt archive/executable and four41f source pins stay unchanged.

Rustfmt's exact PE imports name rustc_driver-737f10a507f50706.dll,
std-f903e3d16df36da5.dll and libgcc_s_seh-1.dll. Its own archive contains no DLL.
The exact58cf compiler distribution source copies compiler runtime libraries into
rustc/bin and adds libgcc plus libwinpthread. This candidate retains the complete
rustc component payload, including its entire bin DLL set, in one fresh prefix;
it does not selectively replace DLLs or execute the component installer. It
requires these four DLLs, rustc.exe/rustdoc.exe and the exact compiler commit.
Actual archive member layout and transitive PE closure are still unobserved.
Presence of those files is not reported as a successful Windows loader test.

Download and unpack are two separate owned phases, each at most120seconds with
128KiB console output. Download is fixed-origin/no-redirect, exclusive-create,
streamed, hashed and read back. The archive is verified again before parsing.
The compressed budget is512MiB, payload/declared expansion2GiB, individual file
1GiB and member count20,000. The decompressed stream, including headers/padding,
has the additional fixed20,000*4096-byte overhead allowance. These are rejection
caps, not observed archive sizes, growth estimates or a promise it fits them.
Paths reject links, reparse/special entries, sparse members, traversal, reserved
Windows names, casefold duplicates and foreign components. The fresh output is
read back with stable path/handle identity and streaming hashes. The original
rustfmt is exclusively created alongside the DLLs, never replacing an existing
file. A bounded local2MiB manifest holds every extracted file's size/hash. Logs
retain its digest/counts, not the complete map. All retained files and both
archives are rechecked after execution; rustc seals are checked even if the later
formatter preparation fails. This is ordinary drift detection, not adversarial
filesystem race isolation.

## Existing gates and limits

Order: existing33controller tests,12stat tests,16new pure/runtime tests; rustc
download; rustc unpack/admission; unchanged rustfmt/source preparation; actual
rustfmt version; unchanged damage3 and terrain4 regeneration calls. The old
run/probe/test changes invert to their complete9846 byte hashes. The two guard
executables, Rust payloads, metadata and immutable historical7139236d artifact
are unchanged. Version uses the one original attempt and raw-child status path.
There is no added process wrapper or second diagnostic execution.

The existing Job assignment/barrier and normal/exceptional ownJob teardown remain
byte-identical. At most3processes means worker -> guard Python -> one sequential
rustfmt. It is sufficient for this gate, not a Cargo build budget. Standard public
Windows job12minutes and step9minutes remain unchanged. Every watched writer
filesystem retains10GiB+128MiB; preflight now additionally requires4GiB for the
fixed preparation caps. The same phase monitor records aggregate observed free
space by volume and Windows GlobalMemoryStatusEx values, with512MiB available
physical RAM floor. Those are measurements and a floor, not a group memory cap.
No privileges, system PATH, registry, installed software or settings are changed.

The previous receipt did not record numeric free space or RAM, so no heavy build
capacity can be inferred from it. Missing layout, budget, DLL/version, permission
or formatter output causes failure without fallback or an automatic retry.

## Proof boundary

The historical artifact is for exact helper regeneration comparison only. This
gate does not run a new planner, check_order, all12guards, apply, Cargo, rustc,
link, assets or a game. The original helper source is not rewritten. Only actual
version and all7matching outputs can make the formatter gate pass. The measured
capacity and successful formatter, if obtained, still do not prove latest complete
composition or BuildOnly readiness. A later build needs the full same-release
official toolchain/WinLibs plus a fresh ordinary stage plan, all guards and apply.

The shared source-CI phase, child-status observer,3process Job and strict named
validator are unmodified. Pure tests use tiny synthetic tarballs, mocked network
and memory calls, exact inverse hashes, missing/changed DLL/commit/path/budget
failures, and original cleanup/barrier regressions. Actual rustc archive layout,
Windows API execution, DLL loading and7formatter outputs remain unexecuted for
this source until independent review and an explicit serial release.
