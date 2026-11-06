# Windows source check: successful corrected retry

Proof head: `5e14cf74e14209f312281b582ee44b7cd97406b0`.
[Run 42233831944](https://github.com/sofia-gros/horizon-mods/actions/runs/42233831944), job `127946186931`, completed successfully on 2026-11-06 at 14:26:13 UTC. The machine was GitHub's Windows Server 2025 / windows-2025-vs2026 image, not a Windows 11 playtest computer.

The real PowerShell parser accepted the entry script without executing it. All exact named pure tests passed with zero failures and zero skips: latest launch 20, historical cache 12, complete-bundle validation 11. The four phases returned zero and recorded empty owned Job Objects after teardown. Their observed durations were 2.484 s, 1.313 s, 0.421 s and 0.407 s respectively. Python was 3.12.10 x64.

The complete files-only source seal covered 6,773 files and was unchanged before/after: `40356bc0bb4c134c9ba029bbe685095749019690dd295e30f2f6aa56c0ea242f`. Empty directories are not sealed. The final receipt has `passed=true`, `verified=true`, `native_executed=false`. It records each exact test name and each phase's argv, output size/SHA and teardown outcome in the companion JSON. The configured continuous disk floor completed without refusal; this receipt does not record an observed minimum-free value, so none is claimed.

The normal GitHub job-log tool returned 18,064 UTF-8 bytes, retaining the original BOM/CRLF, SHA256 `927f74a47ade27c0cf5aadfe1b2e1f43f0d1f8df752f01aa8abc75b351a59a8c`. This identity describes the tool's decoded text, not GitHub's original redirect ZIP. Raw log text is not republished; the full final receipt and log identity are retained in the companion JSON.

## Earlier failure retained

Initial head `c93675e0921c9a2b01bf612a4c6ce4aa45da7f55`, run `42230840423`, parsed PowerShell but passed only 19/20 latest tests, then stopped before historical/bundle suites. One test implicitly decoded UTF-8 stage source as CP1252. Its false receipt remains in `windows-source-ci-first-result-20261106.json`. The correction hashes raw bytes, explicitly decodes UTF-8, fixes the other test-only default text reads and retains the exact CP1252 failure regression. The retry also bounds the final log read before hashing/parsing. Runtime launcher, stage, package implementation, PowerShell source and guard behavior were not changed for this retry.

## What this does not prove

No Cargo/Rust compiler, native packager, engine, game, assets, installer or GPU operation ran. Historical tests inspect the exact reconstructed previous entry rather than executing that entry. Package tests use injected packager stubs. The separately coordinated integration packager/complete-plan proof is not replaced by these tests. Actual current guard admission, Windows GNU compile/link, DLL/runtime operation, asset preparation and game playability remain separate gates. In particular, successful syntax and pure tests are not a successful Windows 11 game launch.
