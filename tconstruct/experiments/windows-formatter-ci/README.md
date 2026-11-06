# Windows1.99 formatter: bounded source candidate

Retry source only, not a Windows build or a current complete-composition check.
Runtime dependency base is aebe7a5d5e778efe273ae1afc2236ff8c3046274, plus the
three stat-reader changes in 3adea9919061e3546b85a4c6668af8fa1547c3c8. The two
existing guards and all Rust helper bodies/metadata are unchanged. The earlier
Linux1.95 guard replay on17f passed damage and both terrain guards; it does not
prove this Windows1.99 formatter produces the same exact strings.

Three native gates only: official pinned archive/layout/executable admission;
actual formatter version; then the unchanged damage verify_source and terrain
verify_regeneration functions. Damage performs exactly three sequential formatter
calls and terrain four. The parent validates observed Popen argv, count and result.
No substitutions or formatting-difference tolerance are added to either guard.

The historical7139236d complete plan is retained only to compare exact damage
helper source. Its immutable four-file artifact is hash/size checked before and
after. This does not run check_order, the complete-plan terrain check, the other
ten guards, a new planner, apply, Cargo, rustc, link, assets or a game. Current
composition success remains a separate prerequisite to the existing BuildOnly
entry. A formatting mismatch is a real failure to investigate, never auto-fixed.

The successful5e14 Windows source-CI phase and strict parser are included byte
unchanged. Its private phase timeout is explicitly120seconds rather than40.
The JobObject code differs only in documented active-process/query capacity2→3:
barrier worker, guard Python, one sequential rustfmt. Assignment precedes barrier
release. Normal and exceptional completion terminate only that own Job and require
it to become empty. Permission/assignment/startup failure is not skipped. Logs
remain bounded128KiB, every writer filesystem retains10GiB+128MiB, and preparation
requires an additional1GiB reserve. Standard public Windows job timeout12minutes;
known phase step9minutes. No service, registry, privilege or global PATH change.

Archive download occurs only in a later released CI: the fixed1.99.1 WindowsGNU
rustfmt tar.gz SHA488f4cdc is checked before bounded safe tar inspection. Only its
one exact executable is copied into a fresh temporary formatter/bin. No installer
or system toolchain is used. The four native source files use exact official41f
URLs/hashes and2MiB each. Unknown source, redirects, archive/member/hash/size drift
reject. The download worker remains under the same ownJob/timeout/disk monitor.

First run42253259752/controller38e1 passed20pure but rejected the cross-API
lstat/fstat comparison of rustfmt.exe before any formatter execution. Its false
result is preserved in571b10f8. The reached code implies archive/layout admission,
but no completed preparation receipt was emitted. Loader dependencies, version
and regeneration remain unobserved. If another DLL/component is needed, this probe
fails at startup and records it; it will not search other Rust installations or
download extra components. Child PATH is only the fresh formatter/bin, selected
Python directory and Windows System32. PYTHONUTF8=1/PYTHONIOENCODING=utf-8 are
process-local. The small known historical artifact action runs between preflight
and post-download validation with2minute step timeout, not inside this ownJob.

Whole checkout, artifact and prepared input seals are rechecked. Failure receipts
retain reached phases/error; missing later gates remain absent, completed=false.
No raw Rust source or credential is emitted. No artifact upload/cache is added.
The native executable's actual hash/version and seven argv records are retained
only if reached. The original guarded source is never regenerated or overwritten.

Twenty controller plus twelve stat-model regressions are required separately,
under the same owned phase, before any preparation download. Both complete merged
verbose streams passed the unchanged strict named validator. They exercise inverse provenance,
ctypes layout, three-process capacity, barrier order, assignment refusal, normal
cleanup, timeout ownJob cleanup, bounds, pins, strict names and admission. They do
not execute Windows APIs, downloads, formatter or Rust. Independent review and
explicit serial release are required before the existing dedicated trigger is
updated. No automatic retry or resource-limit relaxation is included.
