# PR945 playtest: mining-only source intake

This is an unapplied, pure source transformation, not an accepted complete plan or Rust/Game build. It supports exactly two reviewed raw-source inputs: the ten selected files from `e128d61478971e247b1e3e2e09516cf99652f9fc` (PR945 `d4d798fbbdc9dbbe0f81adb0e4074c798737e5ad` plus common), and those same files with the exact Guide B/C manifest postimage `9e7a9e283a8596093ec13cfcab99156ef04e818c8ea9a9ccf65dded6f4e77d5d`, adopted by integration `a057c1ec6e1d50dba438d60a1a011adf5b469350`.

Only **four production paths** change: the TConstruct and Pluto manifests, plus the added `source-mining-context-native.rs` and `mining-context.rs`. Both manifest Horizon requirements explicitly move from 624c to official `41f822b659789780f58d32c63ddfb6887ea1b8bf`. The old two local-speed hooks become one native `native_break_speed` HEAD, and the private borrowed-facts module is appended. Their runtime bodies and declaration are the previously reviewed b78 candidate, not reimplemented policy. The module has `pub(crate)` visibility and borrowed contents remain `pub(super)`.

The chest RETURN declaration and its entire original Option helper are **unchanged**. The owned-ItemStack LOAD proposed in 68bdd/40a0 was rejected by actual official Mixins5a64 (`E_UNSUPPORTED_LOCAL`); it is not included in this intake. Consequently this mining-only intake does not resolve the known chest full-plan blocker.

PR945's shared mining policy remains byte-exact, including `pluto_fluid_facts::player` and current/previous eye-water fields. PR945's melter-store `pluto_inventory::storage::stage_updates` remains byte-exact. The Items mining RETURN, rules/math, old helper files, all other declarations, and both Guide B/C modules stay intact. No 08afa whole-file replacement or historical 646-payload hash is accepted for this different composition.

## Reproduction and boundary

Run `python -B test_adapt.py`: 16 pure Python checks pass. `originals(guide=True)` reconstructs the ten exact source inputs, `transform_selected(..., guide=True)` returns twelve immutable byte values, and `restore_selected` proves the exact inverse. `receipt` records every selected before/after hash. The adapter does not write any tree, change a branch, call Cargo/planner, or invoke native code.

The adjacent source-validation record includes both variants. These are selected-source seals only, **not complete repository or packaged-mod seals**. Integration must match the exact before hashes and new-path absence, apply only the four declared postimages, verify all other tree bytes unchanged, then run the real packager and a separately authorized complete plan/guards/type/build. Raw manifests differ from packaged manifests; do not reuse old packaged member counts or seals.

Prior evidence remains scoped: f4dc mining small probe accepted one HEAD/module formal contract; the finite 36-case rules fixture passed independently; neither proves the new PR945/Guide/41f actor/native composition, visibility, execution order, or gameplay. HEAD precedes native dispatch by design and is not claimed identical to every historical mod ordering. No registry, snapshot projection, Wire clone, save/notification authority or mutable global state is added.
