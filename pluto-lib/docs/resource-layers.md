# Raw data-pack resource layers

`pluto_api::resources` contains host-independent path validation, ordered pack
visitation, and highest-layer selection. It is extracted from TConstruct's
modifier snapshot, rather than introducing another resource loader.

`data_json_path` accepts canonical `data/<namespace>/<path>.json` names. Invalid
names, empty path segments, `.`/`..` path segments and backslashes are rejected.
The namespace retains the source loader's lowercase `[a-z0-9_.-]+` grammar.
This is resource-name validation, not a filesystem sandbox, and opens no files.

`visit_json_pack` takes a caller's file order, reader, classifier and visitor.
The classifier selects resources before any read and may return a routing key.
Only selected, validated names reach the reader. Bytes remain unchanged, even
when they are not valid JSON. An unreadable selected file stops the traversal
with its path; earlier visits are not rolled back, so the consumer must discard
or mark the whole snapshot unavailable after an error.

`visit_json_pack_with_policy` additionally accepts `UnreadablePolicy::Error` or
`Skip`. The original `visit_json_pack` remains strict and delegates with `Error`.
`Skip` omits an unreadable selected file and continues in order; it never proves
the resulting snapshot complete. Both policies retain malformed readable bytes
without inspecting JSON. Selection still happens before reading.

`Layers` is a resource-ID map of raw byte layers. Supply packs from low to high
priority. `highest` returns the last layer and never falls back to a lower file
when the highest file is malformed. No implicit sorting or deduplication occurs.

## Consumer migration

TConstruct's `modifier-snapshot.rs` now delegates these generic operations to
PlutoLib. Its definition/tag path selection, error wording, complete-registry
requirement, condition evaluation, redirect rules, static modifier precedence,
tag merging and canonical identity mapping remain in TConstruct. No resource
reload hook, state owner, source modifier formula or native damage phase changes.

The same feature group also migrates `recipe-snapshot.rs` and
`material-snapshot.rs`. These loaders explicitly select `Skip` to preserve their
existing behavior. TConstruct still strips `recipes/` to form recipe IDs and
selects the last successful raw file for each recipe; an unreadable later file
leaves the prior value, while a readable malformed later file replaces it. The
material loader retains its exact Tinkering directory whitelist, full resource
IDs, definition/tag routing, and ordered layered append. Recipe interpretation,
conditions, material stats/traits, source iteration order and eligibility do not
move into PlutoLib. No cache or global state is introduced.

Standalone TConstruct fixtures import the real shared `resources.rs` source
under their test-only `pluto_api` alias. This avoids copying a second resource
implementation or requiring the native game to test the resolver.

## Integration and validation

The unified API facade must export `pub mod resources;`. The integration owner
manages that facade and package assembly. This branch intentionally does not
edit their shared `lib.rs` or package script.

Focused checks on this branch:
- Four resource API tests passed with `rustc --test` and debug information off.
- Fourteen existing modifier-snapshot tests passed with locked, offline Cargo
  in the condition-snapshot harness; 54 unrelated tests were filtered out.
- A temporary copy of the public API with only `pub mod resources;` added
  passed the four resource tests and the existing unified-API smoke test.
- The broken-melee-damage, material-attack-speed and raging-broken standalone
  harnesses compiled, without executing their gameplay tests. The separate
  raging-melee harness, which needs native host libraries, was updated but not
  compiled in this focused pass.

These checks are not a complete packaged official-host apply/build or gameplay
proof. The combined package validation remains with the integration owner.

The subsequent recipe/material collection slice adds one shared-policy test and
four consumer regression tests, plus the two fixtures' real shared-module
imports. Those additions have not been run individually; they are queued for
the requested combined feature-batch review and tests. Both production modules
are injected only into `game.rs`, so the existing `pluto_api::resources` export
is sufficient and no additional manifest entry is needed.
