# Independent review of the combined PlutoLib source

Reviewed immutable source [035273c4](https://github.com/sofia-gros/horizon-mods/commit/035273c4805d75111e528c8c26607fa3a8d1d5bf). No blocking source-composition issue was found. This is not a combined native compatibility pass.

- All 340 materialized source files match the remote Git blobs. The complete inventory API (33 files) and public API (11 files) remain byte-identical to parent f438f340. The seven terrain, three language/tool, and one Guide Font files match their explicitly recorded source commits.
- All 34 injections, three state declarations and two crate records preserve the prior 8715b123 records and order. Removing exactly the recipe presentation and two language module additions restores the old module sequence and manifest hash. Module file/name owners and section IDs are unique.
- The four composition source checks pass independently with bytecode writes disabled. The existing two archives are byte-identical. All 147 members were read and checked against the actual packager inputs, including manifest and attribution. Archive SHA-256 is `4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d` (270,125 bytes).
- The native stack storage/cells/snapshot and NBT patch helpers are unchanged. The public inventory entry re-exports the original types/functions from the same crate; existing paths remain supported. Staging still means detached proposals, not actor/slot authority, durable commit or rollback. Caller schema, merge/limit policy and notification phases are not moved.
- TConstruct consumers and the separately proven outer160 runtime are not imported. Historical authoring, terrain, language and storage successes do not prove this new composition.

The next required gates remain a complete official plan, all guards, apply, actual host library/binary typing and the separately scoped installed native groups. The recipe presentation module's host typing is explicitly pending. No new live authority, item-name synchronization, retained-book refresh, resource-overlay fix or GPU claim is made.

[Machine-readable review](common-composite-independent-review-20261106.json) records the exact identities and checks. This review ran no Cargo, native job or cleanup and changed no PR state or source. The smallest observed local free-space reading during the review was 10,877,431,808 bytes. The discarded engine fork was not accessed.
