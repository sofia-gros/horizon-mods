# Mining adapter dependency boundary after the scalar proof

Read-only audit of main `08afa992`, common archive `4f13a6e2`, the unapplied
`b78ee288` adapter candidate and official Horizon `f4dcf8a2`. No additional Rust,
metadata, plan, apply or gameplay execution was performed for this note.
The [dependency ledger](dependencies.json) records exact paths, module/state
entries and source hashes. It is a bounded direct-and-one-step dependency audit,
not an exhaustive transitive closure or a new package manifest.

## What the successful 36 cases establish

[Run 42213696048](https://github.com/sofia-gros/horizon-mods/actions/runs/42213696048)
compiled actual native items/core/world and the unchanged scalar Tool/rules/math
sources. All existing 24 plus added 12 cases passed once, with exact inventories,
compiler witnesses, source/lock/binary seals and final verification. Evidence is
[closure 278fe164](https://github.com/sofia-gros/horizon-mods/tree/278fe1643461e1dd84ddecd01079278547ab09ef/pluto-lib/experiments/latest-mining-differential-ci/results/42213696048).

That harness deliberately does not define or compile a replacement Game, the new
BorrowedFacts adapter, or the full source-mining-context-shared actor policy.
Its local re-export uses the exact same Tool rules source bytes, but this alone
does not establish the installed `horizon_items::items::pluto_tool_rules` path.
The actual common archive separately declares that public items module and its
Items::mining_speed / correct_for_drops RETURN hooks. Base Tool evaluation is
packaged there; an actor-context API was not present before this candidate.

## Dependencies needed by the real adapter

The new private Pluto module borrows native StateFacts and the acting held stack,
and returns formal position/max_y. It neither recomputes speed nor selects a
TConstruct policy, stores a frame, mutates a stack, or replaces native dispatch.
The TConstruct HEAD passes those borrows and current mutable speed into its
unchanged shared apply exactly once. `original` is not bound or changed.

The unchanged Game policy and its scalar support require nine Game modules:
- tconstruct_source_mining_context, mining_rules and mining_math;
- tconstruct_source_dwarven and tconstruct_tool_api;
- tconstruct_source_haste_wire and tconstruct_source_haste_source;
- tconstruct_source_material_traits and tconstruct_modifier_snapshot.

Its four directly read ChunkMap states are haste-wire-client,
broad-wire-client, resolved-item-tags and modifier-resource-snapshot.
Material-trait resolution additionally reads material-snapshot and recipe-snapshot,
and depends on the existing world alias, building recipe parser, station stats,
material-trait selector and transaction merge helpers. The ledger records these
five next-level modules and six state definitions. Their further dependencies
must be retained by the complete package, not replaced with test stubs.

The main packager renames generated accessor prefixes from
`__horizon_mixins_tconstruct_` to `__horizon_mixins_tconstruct_native_`.
The ledger distinguishes original-source hashes from rewritten archive bytes.
A full candidate must retain this transform consistently across state declarations
and all their consumers. A small accepted module/HEAD plan does not prove that.

## Latest official borrowing and ordering

The actual f4dc Game::held borrows stack_in(held_slot()); Game::items chooses
world_tables.items when present, otherwise resource Items. The candidate continues
using those methods and the same Game borrow. No alternate actor, global or TLS
store is needed for the proposed facts adapter.

Latest destroy_progress computes tool/harvest and native environment factors,
then calls native_break_speed(pos,state,original,&mut speed), then divides by
hardness and 30/100. The candidate HEAD runs before existing native Mod dispatch,
leaves the immutable original baseline unchanged, and must not coexist with the
two legacy lexical speed hooks. Source order and formal binding acceptance are
separate from applied Game type and runtime actor proofs.

Existing policy distinctions must survive: armor Haste still applies even when
held data is absent/non-TConstruct; malformed Tool is not a valid ineffective
Tool; unavailable Broad tags are not empty; remote chest Haste requires the
matching decoded equipment frame; world material/source eligibility stays
TConstruct-owned. Current eye-water and previous was_eye_in_water serve different
rules. Keeping these sources unchanged does not itself prove latest frame
population, reload invalidation or authoritative guest equipment swapping.

## Gates still needed at this cutoff

1. Complete 646-member plan: confirm exactly the two old context hooks are removed,
   the new HEAD and Pluto module appear once, retained state/module owners and
   generated accessor identities resolve, and subsequent diagnostics are reported.
2. Apply/type the actual combined Game and native module dependency graph. The
   small formal-binding probe and 36 scalar cases are not substitutes.
3. Review composition with native Mod dispatch and other hooks; verify no duplicate
   policy invocation, cancellation bypass, or rewrite of original.
4. Exercise actual actor/frame and local/guest/prediction paths before claiming
   gameplay compatibility. Native zero/NaN comparisons differ by caller; the
   adapter must not introduce a new normalization to hide that difference.

No ordinary manifest is activated by this audit. If full planning stops elsewhere,
that is the next concrete blocker, not evidence that downstream typing or runtime
behavior passed. The tested source and all old proof cutoffs stay unchanged.
