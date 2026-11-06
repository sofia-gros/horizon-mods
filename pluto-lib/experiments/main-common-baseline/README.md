# Unchanged main + successful common bundle: source-only baseline plan

Status: preparation only. No export, index creation, packager, Cargo metadata, official planner, apply, build or test has run for this baseline. This is not a compatibility result and does not change main, a PR, a pin, or any original implementation.

## Exact inputs and known boundary

- Main observed by the coordinator at 2026-11-06T09:05:28Z: `08afa992d5091e03360f3473ea93fd17aa793987`, tree `0c766bc524fc42580c5c7cd04ab71374a4eaf481`.
- Successful common production `035273c4`, CI `0468d4b1`, proof `ae8d4f6b`, independent terminal review `6adba8b0`. Existing complete archive is SHA-256 `4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d`, 270,125 bytes / 147 members. Do not rebuild it from a different branch.
- Main declares Horizon `0c7b5b61` / Mixins `ddd9700b`; common declares Horizon `624c4f62` / Mixins `e8cd0aa3`. Retain both declarations. Do not replace a pin merely to reach a later phase.
- Use only the existing official `624c4f62` export and the official `e8cd0aa3` CLI SHA-256 `50fd3105daafd09525889c11c2374a727dbd41b0bad0a5b8714c01b0d06866ff`.
- The source-derived counts are main 1 crate / 248 modules / 290 injections / 108 states and common 2 / 69 / 34 / 3. Literal target overlaps: 9. Duplicate module `(file,name)` pairs: 0. The inputs JSON records every overlapping target and IDs. These are static observations, not a resolved complete-plan result.

The actual planner resolves hooks, loads its index using metadata and plans native crates before it compares Horizon revisions. The first real rejection may therefore occur before `E_REVISION`. Preserve the actual diagnostic, order and target. The archive header's Mixins pin is recorded separately: feeding raw mixin sources to the planner does not prove launcher acceptance of that header.

## Small exact source closure

The original `tconstruct/tools/package_hnmod.py` uses `git ls-files`. Its `tracked()` generator checks every file before the outer code filters documentation and tools. An 18.3 MB export containing only the eventual archive members would be invalid.

The exact required input closure is 5,435 regular mode-100644 blobs, 44,391,808 payload bytes: all tracked files under the seven prefixes in inputs.json plus `tconstruct/compatibility.json`. This saves 12,312,732 bytes versus the full 56,704,540-byte main tree. Materialize only those pinned Git objects, streaming `git cat-file --batch`, and verify path, object ID, mode and byte count. Do not borrow current working-tree files or let Git export-ignore attributes silently omit inputs.

The full input seal can be reproduced from `git ls-tree -rlz <main>` in that order, selecting the recorded prefixes/additional file and serializing records containing path/mode/type/oid/size using JSON sorted keys and compact separators. Its SHA-256 is `bfd3cb30ceb9d6ebb05703ab75bc7ba1395407d694ae7d2a5a708a51b705ca47`. The complete locally retained seal is in the preparation directory. The official export is also sealed, including the exact `.horizon-revision`. Its only committed source symlink, CLAUDE.md→AGENTS.md, is the already verified portable regular-file materialization, not an unrecorded source rewrite.

## Isolated index, packaging and complete bundle composition

A later controller must create an entirely new work directory under `/tmp`. It may use the existing authorized MOD object database read-only, with `GIT_DIR` pointing to `horizon-mods/.git`, but set a new absolute `GIT_INDEX_FILE` and `GIT_WORK_TREE` for every child. Seed that separate index with exact main via `git -c core.splitIndex=false -c core.sparseCheckout=false read-tree <main>` without `-u`; never touch the original index or refs. Disable optional Git locks and bytecode writes. Verify the isolated index contains the exact main entries and the original refs/index are unchanged.

Run the original main packager verbatim against this source/index pair. Its existing native-ID/accessor rewriting is part of the fixed packager behavior; no additional runtime or pin rewriting is allowed. Keep the complete produced `.hnmod` and its header. Independently reproduce expected member bytes from the exact inputs, including the nested support ZIP. Validate unique/safe member paths, entry limits, regular files and exact uncompressed byte totals.

Extract every native `mixins/` member from that artifact and the complete successful common archive into one fresh combined directory. Both owners remain distinct. Reject duplicate paths rather than overwriting them. Retain both complete archives and their conflicting headers separately; do not unpack the nested inert support data as active mods.

## Bounded command authority

The prepared metadata wrapper differs from Recipe's proven wrapper only in its new log destination. It permits exactly:

`metadata --format-version 1 --no-deps --manifest-path /workspace/scratch/c7b540ff3393/.devtools/latest-official-export-20261105/horizon/Cargo.toml --offline --locked`

All other Cargo arguments exit97. The separate rustc wrapper permits only `--version` or `-vV`; all compiler invocations exit97. Use the existing real toolchain binaries directly, with no rustup/install/network operation. Set both CARGO and PATH to the metadata wrapper and RUSTC/PATH to the version wrapper.

Use a fresh private CARGO_HOME, TMPDIR and nominal target path under the new `/tmp` work directory. Do not copy/link current registry caches or build outputs. No-deps metadata is expected to inspect the workspace; this expectation has not been executed. If an empty CARGO_HOME is insufficient, stop with the exact metadata error. Do not silently fall back to the protected current cache, resolve online or build anything.

After separate review/grant, the only planner command is the exact existing CLI with `plan --workspace <sealed official export> --mods <complete combined mixins> --out <new work>/plan.json`. There is no apply/check/test continuation, even if the plan unexpectedly succeeds.

## Capacity and terminal contract

All new generated files must remain under the fresh `/tmp` directory, capped at **192 MiB** of allocated space. Budget: export+directories72 MiB; separate index4; complete archives20; combined payloads24; possible plan32; logs/metadata/receipts/seals8; reserve32. This budget is not a measured runtime peak. Recheck actual file allocation at every phase and continuously while commands run. Keep the overlay above 10 GiB + 128 MiB and tmpfs above a separate 2 GiB reserve. Bound every output and command duration; preserve complete stdout/stderr rather than only a failure tail. Children must run in an owned process group, receive bounded termination on failure/floor/budget/timeout, and be absent before any follow-up.

Before and after each phase, verify source/official/CLI/wrappers; before terminal, also verify original MOD ref/index, isolated index, both archives/headers and combined members. No current cache, source, evidence or unpublished work may be changed or cleaned. A mismatch, first command failure, unexpected subprocess, capacity condition or unknown provenance stops the attempt with a false/pending receipt. Do not call an intended E_REVISION an executed result.

Later work, if separately authorized, would address pin compatibility and the recorded exclusive placement/matching target overlaps. This source preparation neither repairs them nor claims full TConstruct or original-PR compatibility.
