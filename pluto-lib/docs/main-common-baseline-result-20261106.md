# Unchanged main + common: actual planner rejection

[Run 42198595853](https://github.com/sofia-gros/horizon-mods/actions/runs/42198595853), job 127827974965, exact source 7389a163, completed the bounded diagnostic. **Compatibility is false.** The GitHub job's success means the diagnostic and source-preservation checks finished, not that TConstruct works with the new common bundle.

The one actual official planner exited 1 with E_REVISION for `tconstruct_native/horizon-mixin.toml`: it requires Horizon `0c7b5b61a5e746d621afffba919b12aeccf9c0fa`, but the exact official target is `624c4f6265c02296fbe8a80f029dd0d987fb94c0`. This is the measured first rejection, not a substituted prediction. Main's old Mixins header pin is retained as separate evidence and was not itself the measured diagnostic.

Inputs remained unchanged: main 08afa992 was independently checked out and packaged with its own packager; common production 035273c4 reproduced the exact successful 4f13a6e2 archive. Main 499 entries including 4,662 inert support members and common 147 members produced all 644 native payload entries without overwriting collisions. Both original headers and complete source/Git index/refs/HEAD/artifact seals passed final checks.

Eight required diagnostic gates completed. Twenty pure Python checks passed. The official Mixins CLI alone cold-compiled, with an actual `horizon_mixins` compiler witness. It then made exactly one allowed offline/locked/no-deps metadata call under the rejecting Cargo/rustc wrappers. Of 22 phases, 21 exited 0 and the one actual plan exited 1 as recorded. `diagnostic_complete`, `source_verified` and `probe_completed` are true; `plan_accepted` and `compatibility` are false; controller error is null.

There was no retry, pin change, complete-plan guard execution, apply, Game/engine/consumer build or runtime test. The later shared-target and exclusive-guard questions remain unresolved because the revision boundary stopped this attempt. Original PRs and main were not changed.

Remote minimum disk free was 90,545,958,912 bytes; minimum observed available memory was 13,606,666,240 bytes. The unchanged safety predicate observes host and enclosing cgroup limits; it does not install a hard cap. The local environment had already failed memory admission and performed no local packaging/planner/native fallback.

The companion JSON embeds the complete retrieved console, its compressed preservation hashes and the complete final receipt. Verbose phase logs and artifact ZIP bytes were ephemeral; their hashes do not reconstruct them. Raw-source planner output is not launcher-header acceptance or full application/runtime compatibility.
