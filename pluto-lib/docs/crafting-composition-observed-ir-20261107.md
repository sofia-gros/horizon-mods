# Crafting composition: observed constructor and module metadata

This supplements the historical source-only [guard contract](crafting-composition-guard-contract.md) and [source receipt](crafting-composition-guard-source.json). Those earlier receipts remain unchanged. The new [bounded observation receipt](crafting-composition-observed-ir-20261107.json) records the exact inputs and the three metadata/test changes.

## Observed input, not a new runtime change

- Integration input: `e9073648b3c4cf4d49f27bc2e1ed60997f430586`.
- Official Horizon: `41f822b659789780f58d32c63ddfb6887ea1b8bf`; Mixins: `5a64af938ec86b00d53d16a408aabc0038fb3e57`.
- [Actual complete-plan run](https://github.com/sofia-gros/horizon-mods/actions/runs/42328100345), controller `7082868bcbf37ed3eabd32f40ddf0cf08a4ee30c`.
- Plan: 6,431,102 bytes, SHA-256 `dc4fbc3c54144f2f95728986b4e9bfb3700235655b18a3af3c8d1a6113dcb86b`, accepted in 460.676 seconds. The original run reached all twelve guards and refused four, including these two crafting guards. This update does not rewrite that result.

The original result is preserved in the [published plan receipt](https://github.com/sofia-gros/horizon-mods/blob/c50b6acbe7f7eebe1032149dbcbf8cf6033b61f4/pluto-lib/docs/playtest-next-plan-result-20261107.json).

No Rust, manifest, checker, API, package-contract schema or caller interface changes are made here.

## Two constructors now have actual identities

Both hooks belong to `tconstruct_native`, target `horizon_client_lib::crafting::Recipes`, and remain priority 100, noncancellable HEAD, empty bindings and a void contract. Their helper bytes match the ordinary source exactly.

- `empty`: node `[42, 0]`, fingerprint `1e84c586bff87f44d88440bc32d0bb8f72a140325977f61a302e893dbcc4729c`.
- `from_sources`: node `[42, 4]`, fingerprint `10979e222d48a73a212516bb5f05b977d4ee9fedf4a4c840a27eb2ec10b8dd7b`.

Only the four formerly null node/fingerprint fields are filled. The four existing assembly, remainder and placement boundaries remain byte-for-byte unchanged. The adapted existing test still rejects missing observations and changed fingerprints/nodes; it does not substitute a permissive rule.

## Crate visibility is omitted in serialized IR

All five affected module declarations explicitly request `visibility = "pub(crate)"` in the manifest. In official [plan.rs](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/plan.rs), `PlannedModule.visibility` uses `skip_serializing_if = "manifest::ModuleVisibility::is_crate"`. The official [manifest.rs](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/manifest.rs) defines the default as `Crate`, serialized as `pub(crate)` when it is present.

Thus the actual plan omits the visibility key. The old closure incorrectly copied the manifest string into the expected IR field. The fix records null, matching the unchanged checker's `.get()` comparison, for these five modules:

- `pluto_transactions`, in both standalone-Pluto and full-Tinkers metadata.
- `tconstruct_crafting_deny_policies`.
- `tconstruct_crafting_bootstrap`.
- `tconstruct_overslime_derivation`.
- `tconstruct_station_transaction`.

This is six metadata occurrences, not a visibility change. No general visibility normalization is added. An explicit public module remains a mismatch, covered inside the existing synthetic closure test.

The complete plan's caller sources were inspected: the crafting policy/derivation registrations and constructor helpers stay within the library's crafting parent/siblings. The placement helper and station handler stay in the Game crate; `game.rs` calls the station module's `pub(crate) ensure`. None of these five modules was an external-crate public facade. Actual Rust privacy/type checking is still pending.

## Retained checks and limits

The actual plan's six hook identities/helpers, sixteen dependency modules and their full source hashes, six native input hashes, exact inventory crate, station state records, separate LAN hook and protected hook/module sets match the corrected metadata. The two old Overslime library module bodies and the old station transaction body were independently compared with the earlier `7139236d...` complete plan: the former remain byte-exact and the latter remains the exact single suffix of the new handler.

All 43 source/synthetic tests passed after restoring their immutable dependencies into a small temporary source view. The same two actual packaging tests remain present but were not executed. The source-view setup initially lacked supporting checker/JSON files; those immutable files were supplied without changing any test or runtime behavior.

This is a source and saved-IR comparison, not a complete guard rerun against fresh local archives and a pristine official checkout. The original run independently checked its real archive contract and official sources. Re-executing the two updated guards, then all mandatory guards, apply and host compilation is the next gate. Constructor execution, registration behavior, Game typing, Windows launch and gameplay are not proven by these observations.
