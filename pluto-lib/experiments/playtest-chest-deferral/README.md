# Temporary pick-block color deferral

The user explicitly approved temporarily disabling only pick-block color inheritance so the integrated playtest can proceed. Stored chest color must remain intact, and the feature remains a restoration task.

This intake removes exactly the `tinkers-chest-pick-color` RETURN injection from integration commit `24b8ff1671db1ed2fb3c0d0a9c0c952f6f0211fc`. It does not change the ordinary `source-chest-pick.rs` helper, chest metadata reader/writer, NBT schema, stored data, other injections, modules, states, crates, or save/notification behavior. The native pick-block result remains native; the removed behavior is the extra TConstruct stored-color override. Existing items or world colors are not erased.

`adoption.patch` changes one manifest only. `intake.json` preserves the complete removed block, offset, before/after byte identities, and unchanged helper identity. `apply_exact` requires the exact source and helper; `restore_exact` reconstructs the original complete manifest. These functions do not perform filesystem writes. The coordinator owns the integration branch and must verify every unrelated tree entry is unchanged when adopting this delta.

The previously proposed nonprimitive LOAD was actually rejected by Mixins, and the unchanged RETURN also encounters the outer-try restriction. This deferral is neither a hook repair nor proof of full compatibility. Restore the block only after a supported result-edit boundary is implemented and independently checked for early None, full signed color range, nested metadata precedence, and unknown stack data preservation. Restoration on a later manifest requires a new explicit reconciliation, not blindly applying this old inverse.

Validation: 12 pure synthetic/intake checks and an actual immutable before/postimage byte inverse. No planner, apply, Rust, or gameplay execution was performed for this intake. The ordinary helper remains in the package as inactive source for restoration.
