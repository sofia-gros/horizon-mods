# Windows latest launch source-only gate

Status: prepared, no Windows execution yet. This is not a game build, latest plan, package generation, runtime or GPU proof.

Base: `59f3fa23cc5b95fbc8e7bcb42d24e2d5d9771bc9`, the exact Windows v3 source plus expected ZIP-warning capture and actual stage baseline LF correction. The launch implementation pins official Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf` and Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`, but this gate does not download, inspect or compile either repository.

Only four phases run, on the standard public Windows runner:
1. Existing Windows PowerShell Parser.ParseFile reads `play-windows.ps1`. It does not execute the entry.
2. All20 latest admission/source tests.
3. All12 historical cache tests. Their historical PowerShell body is reconstructed by its exact inverse witness; it is not executed or claimed to test the new launch body.
4. All11 synthetic bundle tests. Both packager processes are stubbed; no actual integration bundle is produced.

Exact names/counts/zero skips and normal unittest trailer are required, including CRLF handling. The intentional duplicate ZIP warning is caught and asserted inside that negative case, not globally hidden. Local12 controller regressions include running all43 real verbose merged outputs through this validator, low-disk no-spawn, failed-assignment barrier and normal cleanup mocks. No Windows API, PowerShell or native process was run in those Linux checks.

The workflow is exact-branch push-only, public `sofia-gros/horizon-mods`, contents-read, pinned checkout, no saved credential, cache, artifacts, installer, elevation or new permissions. Existing Python3.12 x64 is required; absence fails. Checkout receives process-only `core.autocrlf=false` to preserve frozen source bytes; no global/local Git setting is written by this candidate. Existing workflows are unchanged and use another exact branch, PR, manual/call or tag triggers, so this branch does not request unrelated jobs.

The job timeout is5 minutes and controller step timeout3 minutes. Each phase is limited to40 seconds and128KiB merged output. The controller checks10GiB+128MiB free on every filesystem used by source, isolated runner temp, drive root and Python at admission and every50ms. This is observed free-space admission, not a hard storage reservation. The phase's own temporary logs and synthetic files are cleaned, never user source or assets. The full checkout file-byte map (excluding only `.git`) is compared before/after, including untracked/ignored files, with links/reparse/special entries rejected. Frozen source files are separately pinned in inputs.json. This is a file-byte seal. Empty directories, directory presence and directory metadata are not sealed.

Each phase creates one own worker behind a stdin barrier. Its CreateProcess handle is assigned to a fresh Job Object before the barrier is released; no PID is reopened or discovered. KILL_ON_JOB_CLOSE and a two-active-process limit cover the worker plus its one fixed parser/test child. A failed assignment stops before the child, kills only the waiting worker and fails the gate. Normal and exceptional exits terminate the own job, wait for its leader and require no assigned processes within2 seconds. Job Objects are normal user operations, not permission/configuration changes. Unavailable Job Object assignment or symlink privileges fail visibly; there are no skips, permission fixes or uncontained retries. The process count is not a hard aggregate RAM limit.

The compact FINAL_RECEIPT always distinguishes source verified from all gates passed, includes partial phase argv/exit/timing/log bytes/hash when available, and records exact accepted test names. Failure tails are bounded. Raw phase logs are ephemeral and not uploaded. Controller test code is excluded from the four Windows phases; its source and12 Linux regressions are preparatory evidence only. No claim of latest complete-plan admission, guarded apply, Cargo/engine build, resource loading, save correctness or playability follows from this gate.

Publish first to a non-trigger checkpoint. The trigger branch `checkpoint/windows-latest-source-ci-20261106` must remain absent until independent review and the coordinator's explicit release. No automatic retry or resource-limit relaxation.

## Initial actual run and proposed correction

The first c936 run reached Parser success and latest19/20, then stopped on implicit locale decoding in one test. See `tconstruct/docs/windows-source-ci-first-result-20261106.{md,json}`. This follow-up changes only the test read to bytes/explicit UTF-8 plus its exact CP1252 reproduction, pins and documentation. Runtime59f3, controller/workflow/limits/strict parser and all43 test names are unchanged. The earlier11 regressions, including actual43 merged output, passed; the following controller hardening raises that preparation inventory to12. Retry requires independent review and a new explicit serial release.

The scoped three-suite follow-up also makes the remaining eight ASCII-source/JSON reads explicitly UTF-8 and checks that source policy in an existing test. It does not set Python UTF-8 mode or change OS locale.

## Bounded final log read

After a follow-up review, final phase-log hashing and parsing now check size before opening and read at most128KiB+1, rejecting growth or a size change. This closes a final-read boundary even if a process exits after writing beyond the last polling check. One pure regression covers pre-open oversize, growth at read and a bounded normal file. Initial actual phase logs were13/3142 bytes, so this does not reinterpret that failure. The source seal is files-only; no empty-directory presence claim is made. Runtime,43 suites, Job Object policy, workflow and limits are unchanged. Windows retry remains unrun.
