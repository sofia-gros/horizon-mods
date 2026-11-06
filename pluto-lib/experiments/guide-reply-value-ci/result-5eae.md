# First reply-value CI attempt: stopped after compilation

Exact source `5eae8f5427c30f755af398439cacf321086bfdf0`, [run 42239364026](https://github.com/sofia-gros/horizon-mods/actions/runs/42239364026), job `127965267342`, failed at 2026-11-06 15:03:53 UTC. No automatic retry was made. The original source and false receipt remain fixed.

The 24 runner, 21 preparer and six source checks passed. The actual fixture was prepared; Cargo resolved the intended single workspace member/default member, and a second locked metadata query matched the first (39 packages). The actual `cargo test --lib --no-run` returned exit 0.

The subsequent executable-artifact parser failed with `JSONDecodeError: Expecting value: line 1 column 2 (char 1)`. It currently parses every stdout line as JSON. A verbose Cargo build-script prefix is a plausible source, not an observed diagnosis: the raw build stdout/stderr stayed on the runner and were not recovered. Their exact sizes and hashes are in the JSON. The earlier compiler parser returned before artifact admission, but its witnesses were not printed or retained in the receipt, so they are not independent compiler evidence here.

The executable was not admitted/sealed. The 21-name inventory, ten selected reply tests, and binary/OUT_DIR final checks were not reached. The final receipt has `passed: false`; its `verified: true` covers the recorded source/lock/export final checks, not successful tests. The full decoded job console is held locally, 46,313 UTF-8 bytes, SHA256 `27647ec55fe928414b9241b70cc2dbf19b306e3cc1b6a96704972d8c52f675a2`.

No ordinary hook, Game/UI, retained lifetime, observer ordering, transport or LAN behavior is proved. No runtime change is included in this results-only checkpoint.
