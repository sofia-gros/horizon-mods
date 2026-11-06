# Outer tank: exact three-hunk intake

This adopts an existing boundedly tested delegation, separate from PR945's `source-melter-store.rs::write_many` use of `storage::stage_updates`. It does not edit that file or replace the whole PR945 implementation.

Input is the exact two files at integration `a057c1ec6e1d50dba438d60a1a011adf5b469350`. Output is byte-identical to their already validated source at `794a0c526ca0a96d0ca9c9ff40010611b5968a89`. Full-file comparison found precisely three differences, recorded in `intake.json` and the readable `adoption.patch`:

1. `source-fluid-cannon.rs`: `tank_to_nbt_preserving` final compound patch delegates to `pluto_nbt_storage::patch_compound`.
2. `source-machine-runtime.rs`: the Drain's outer controller proposal uses the same helper before the unchanged revalidation/write/held-item path.
3. In the same file, `patch_smeltery_controller_tank` preserves the initial shape guard and uses `once_with` so tank encoding remains after the helper's original compound clone.

No other source bytes, metadata ownership, save order, notification, held-item update, schema, wire policy or fallback behavior changes. The existing `nbt-storage.rs` dependency in a057 is byte-identical to the historical fixture (`aa4c33e9e24ca9128bb8e8378684ce3138ceb663301aa18b35344eecda9aaaf8`). No new API or manifest is required.

## Evidence and limitations

Historical proof [a09e0d48](https://github.com/sofia-gros/horizon-mods/commit/a09e0d48d9ddba4dd872a6837912f3da2783d539), run42186716846/job127788705395, passed 160 actual Rust cases, including four new named cases, against official **624c**. Source runtime was 794a; code originated in 8dacbcf6. Drain used the existing mock world/Game. This proves the finite actual NBT/ItemStack boundary, not an installed Game, disk durability, arbitrary actor authority or whole-client behavior.

The current selected source before/after hashes and full byte inverse were checked locally without Cargo/native execution. Wrong source bytes, a wrong dependency, altered postimages and repeat application are rejected. Full source hashes mean no unrelated PR945 change can be silently discarded. `intake.verify_inputs` returns immutable bytes and never writes a repository.

**41f Rust/Game typing, complete plan, installed behavior and playability are unverified.** Incorporation into the coordinator-owned branch must change only these two postimages, preserve every other path, and acquire the actual new full package/source seals. Do not label the old 160-case proof a latest-official or full-composition result. No new trigger, planner, metadata or build is included here.
