# Outer tank playtest source composition check

Integration commit `24b8ff1671db1ed2fb3c0d0a9c0c952f6f0211fc` was independently read and compared with its `cb7909c4` parent and the frozen `b12a8d4b` intake.

- Exactly two ordinary runtime files change, with three hunks: `source-fluid-cannon.rs` and `source-machine-runtime.rs`.
- Their Git blob identities match the exact intake postimages and historical runtime `794a0c52`: `36fa892a0fa615ceb24e22ef5c74f160dd2e8d89` and `b48515995bf47eb9a63b95b9eb62012a59f8286c` respectively.
- The selected input hashes, all output bytes and the complete selected byte inverse were rechecked. Existing `pluto_nbt_storage` remains exactly the old160 dependency.
- Ordinary manifests, old chest RETURN/helper, PR945 shared mining policy, melter `stage_updates`, Guide modules and mining adapter are unchanged by this commit. Added crafting files remain in the experiment directory, without manifest wiring.

This is **source composition evidence only**. The historical a09e0d48 proof passed 160 tests on official624c and used a mock Drain world/Game. No latest41f plan, Rust/Game type check, installed execution, durability or playability is established here. No Cargo, metadata, planner, apply, build, cleanup or new trigger was run for this check. The coordinator retains exclusive ownership of the integration branch.

The adjacent JSON preserves exact hashes, the three actual commit hunks and previous-parent identities. A future complete-package/mandatory-guard/build test must acquire new source and artifact seals rather than relabel the old fixture proof.
