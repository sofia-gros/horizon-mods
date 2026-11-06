# Latest official, revision-only main/common diagnostic

Source-only preparation. No new workflow trigger, metadata, planner, apply or
native execution has been performed for this cutoff. A revision edit is not a
compatibility repair. Original main, PRs, source bodies and earlier proofs remain
unchanged.

## Fixed inputs and exact scope

- Original MOD main: `08afa992d5091e03360f3473ea93fd17aa793987`.
- Original TConstruct package: SHA256
  `8178b440266664d3db4dd9784af02894ebdb29910b0dd8f13504b2d341cf8f42`,
  499 members, recreated with that main checkout's unmodified packager.
- Original common bundle: SHA256
  `4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d`,
  147 members, 270,125 bytes. Production `035273c4`, historical successful
  common CI `0468d4b1`, proof `ae8d4f6b` apply only to official 624c/e8.
- Latest official Horizon fixed at
  `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd` and Mixins at
  `5a64af938ec86b00d53d16a408aabc0038fb3e57`.
- Historical unchanged-composition diagnostic: runner `7389a163`, proof
  `08bf25d9`, actual `E_REVISION` rejection. That run is not relabeled.

After exact recreation and complete verification of both original archives,
compose an immutable original 644-member payload tree. Create a separate
candidate tree by changing **only two** `compatibility.horizon_revision` fields:
TConstruct 0c7 to f4dc, and common 624c to f4dc. Preserve all other 642 entries and
all other bytes in those two manifests. The old/new line bytes, complete
manifest sizes and SHA256 values are fixed in `official-contract.json` and
recorded in the receipt. A full inverse comparison verifies every candidate
entry against the original payload before writing the candidate.

Neither archive header is rewritten. Their raw old Horizon/Mixins fields are
reported separately from the actual latest tool and workspace identities.
TConstruct's header still says Mixins ddd9700b and common's says e8cd0aa3. The
official plan reads the unpacked manifests; it does not perform packaged-loader
header admission. This diagnostic therefore proves no new usable `.hnmod`.

## Read-only official contract audit

`official-contract.json` records exact URLs/revisions, Git blob SHA and SHA256
for eleven selected files, independently checked against untruncated official
Git tree responses. Both latest checkout contracts are rechecked before tool
build. Mixins Cargo.lock, root/CLI manifests, CLI options and diagnostic schema
are byte-identical to old e8. Index now canonicalizes the workspace and resolves
crate self-aliases; the exact metadata argument sequence is unchanged:
`metadata --format-version 1 --no-deps --manifest-path <official>/Cargo.toml
--offline --locked`. Latest Horizon adds horizon-mod-deps and changes its lock.
No old lock graph, host type proof or runtime result is transferred to latest.

The old portable export helper is retained unchanged. A new adapter permits
only the pinned latest Horizon and creates its explicit `.horizon-revision`
marker after rejecting an existing marker. The selected official source has no
such tracked marker. No source body or original checkout is edited.

## One-shot remote flow and safety

This is a minimal source-only derivative of successful `main-common-ci`.
Reuse its immutable preparation helpers, memory predicate and the successful
common Runner's process-group/timeout/disk/seal implementation. The copied
memory helper's older wording about native entry applies to Game/consumer code:
**only the official Mixins CLI tool may be cold-built in this diagnostic**.

Use a new exact public trigger branch, read-only workflow permissions, pinned
checkout action, no saved credentials, external cache, upload/deploy, settings
change or added billing. Private fresh Cargo home and target are job-owned.
No local execution is admitted. Visible memory below the existing thresholds
or ambiguous cgroup topology fails closed. Initial disk reserve is 4 GiB above
the 10 GiB + 128 MiB running floor. No memory/cgroup/security settings change.

After the actual official CLI compiler witness is recorded, install strict
metadata-only Cargo and version-only rustc wrappers. Every unexpected argv is
refused before execution. Run exactly one official plan. Preserve the actual
complete diagnostic JSON rather than expecting any particular error code.
Do not retry, repin again, apply, build Game/consumer code or run gameplay.

If the plan succeeds, keep the old common planned-payload validation and the
existing twelve guard groups byte-identical. The unchanged aggregate stops at
its first rejection. Record that exact stderr/exit and do not claim later groups
ran. Guard rejection gives `compatibility=false`; passing all guards leaves
runtime compatibility `null`, because this remains plan-only. A memory/process
failure is an incomplete diagnostic, not a compatibility result.

Git refs/index/HEAD/config, every original checkout including ignored files and
symlinks, original and candidate payloads, wrappers, archives and CLI bytes are
sealed before/after. Regular-file mode and directory presence/kind are checked;
directory permission modes are not claimed. Own process groups must be gone.
Full receipt distinguishes `diagnostic_complete`, `source_verified`,
`probe_completed`, `plan_accepted`, guard outcome and `compatibility`.

## Verification status

40 pure Python regressions passed for this source cutoff (0.103 seconds), including exact pin and
source guards, wrapper refusal, one-shot rejection, two-manifest byte inverse,
all 644 entries, untouched archive headers, explicit latest export marker,
guard rejection versus safety interruption, and unchanged memory function
bodies. These source tests do not execute Cargo, native code or the planner.
Publish a non-trigger checkpoint and obtain independent review and a separate
serial release before creating the trigger.


## Post-timeout observability-only follow-up

The first latest run at db543107 timed out in its one 180-second plan. Complete
console/receipt are immutable at ac311972; compatibility remains unknown. Its
old collection occurred only after the plan returned, so wrapper invocation
records were not available in the failure receipt.

The e28b972b follow-up collects the existing wrapper log in finally, even after timeout
or another exception. It validates every record against the same exact argv and
real tool paths, and includes raw byte count/hash and file mtime-ns. Reads are
limited to one regular non-symlink file, 64 KiB and 64 complete JSON lines. A
partial/unexpected record is a separate verification failure. An absent file
means no recorded invocation witness, not evidence that metadata completed.
These are pre-exec call-start records only; metadata completion and planner
subphase/target remain unknown without further evidence.

At e28b972b, wrapper generation/permissions, tool/source pins, both manifest
changes, 180-second timeout and safety/native guards stayed unchanged. No CPU
observation was added at that cutoff. It was preserved without a retry.


## Separately approved 600-second bounded observation

This later source cutoff changes only the official-plan timeout to **600
seconds** and routes that one call through `observe_plan.py`. The shared old
Runner body, native CLI bytes, wrapper semantics, source/lock identities,
manifest candidate, memory/disk/process-group guards and twelve native guards
remain unchanged. A separate review and execution release is required. No
local planner/native execution or retry has occurred for this source.

The observer is the unchanged Runner's owned session/group leader. It starts
one CLI child in that same group, retaining the CLI's exact stdout/stderr. A
separate JSONL file samples only that Popen-known direct PID every five seconds,
at most 128 samples and 64 KiB. No process table scan, descendant aggregation,
command line, environment or unrelated process data is collected. `/proc/PID/stat`
is read only to verify PPID/PGID/session/start-time identity and retain numeric
user/system CPU ticks. Its process-name field is discarded. Each sample
rechecks identity and nondecreasing ticks; unknown/live-unreadable or reused PID
identity fails rather than silently accepting another process.

A missing stat file is treated as normal exit only if the same Popen object
confirms and reaps completion. Otherwise it is an observation error. Early exit
can legitimately produce no CPU samples. Sampled ticks are not final total CPU,
CPU percentage, metadata completion or evidence of an internal planner target.
Observer and CLI exit values are recorded separately; success/expected rejection
requires both to agree. Expected CLI exit 1 preserves the original diagnostic
bytes without observer text in that stream. Observation failures exit 98 and
are not parsed as official diagnostics.

The outer Runner owns the 600-second limit and kills/verifies the whole owned
group, including the observer, CLI and any subprocesses. SIGTERM is handled only
to reap the known CLI child with bounded waits before observer exit; no timeout
is extended. Finally collects both
bounded telemetry and wrapper-start records after teardown, even on timeout.
Timeout keeps compatibility null. No automatic retry, guard relaxation, apply,
Game/consumer build, main or PR mutation follows. Source regressions mock Popen
and proc observations; they do not start native code. Two older test doubles
needed an explicit work path after the observation filename was added; both
were fixed during source preparation, before any native execution or freeze.
