# Ordered predicates with unknown results

`pluto_inventory_api::predicate_fold::{any, all}` lazily consumes
`Option<bool>` values in input order. No schema, leaf evaluation, registration,
native hook, global state or game ownership is included.

- `any`: the first known true settles the result. Otherwise unknown is retained;
  an exhausted iterator without unknown returns false, including an empty one.
- `all`: the first known false settles the result. Otherwise unknown is retained;
  an exhausted iterator without unknown returns true, including an empty one.
- The unneeded tail is not visited. Callers should pass a lazy iterator to retain
  that behavior; eagerly collecting leaf values has already evaluated them.

The exact repeated bodies come from MOD `91fa95fd`: conditional recipe AND/OR,
`SourceCondition::And/Or`, and melting `Ingredient::Any/All`. Only those five
fold bodies delegate to the shared implementation. JSON parsing, nonempty-array
validation, Forge/Mantle/Tinkers IDs, conditions, tag availability and leaf
semantics remain in the consumers. The stricter station repair parser is not
changed: its unknown propagation is different and must not be normalized.

The native consumer path is the existing `horizon_client::pluto_inventory`
reexport. Official `client_lib.rs` already aliases itself as `horizon_client`,
so the same consumer source can remain at both existing host module locations.
No new crate, manifest hook or dependency between bundled crates is added.
The packager's existing complete Rust-source traversal includes the new module.

## Focused evidence and limits

`tests/predicate-fold/legacy-oracle.json` freezes the original body bytes and
their full source-file hashes. The source guard reconstructs those files by
undoing just the five delegation edits and checks that the strict station file
is untouched. A real-Cargo-package differential fixture compares all 6,558
result-and-visitation cases across three callers, two operators and all sequences
of length zero through six. Its leaves represent already-resolved true/false/
unknown values, not native parsers, registries or game state.

Run the fixture with `cargo test --locked --offline --manifest-path
pluto-lib/tests/predicate-fold/Cargo.toml`. Run the complete inventory API tests
through the existing locked runner when resources permit. Current execution
status is recorded separately in `predicate-fold-validation-20261106.json`;
prepared tests are not passed tests.

The condition-snapshot and soul-glass Cargo harnesses now declare the actual
local inventory API dependency. All seven known source include sites reexport
that real crate. Six stand-alone Rust files do not all have committed runners;
an external runner must link the actual crate rather than insert a stub. Their
broader casting/acquisition behavior is not established by the pure fold proof.

This extraction neither enables automatic crafting nor implements the unapplied
pair-authority proposal. Existing source-policy wrappers keep their previous
activation status. Complete TConstruct migration/gameplay remains separate.
