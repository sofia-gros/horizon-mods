# Independent review of mining candidate 6c0a3f64

Reviewed source cutoff: 6c0a3f64aec3a3c8f2a7798ea3e2da37cae9309f, tree d90a6e93fa2360012ff51be453598d9cfb308d64. The eight declared source hashes plus inputs.json were re-read locally. Review did not edit those nine candidate files.

No serious concrete type/semantic defect was found in the bounded source review. This is not a compiler or planner result.

## Checked

- Pluto's BorrowedFacts ties StateFacts and the optional actual ItemStack borrow to the same borrowed Game lifetime. It keeps the formal position and current world max_y without cloning or projecting the stack/components. It does not supply a mutation or actor authority token.
- The helper calls read(game, pos, state), then the unchanged TConstruct shared apply once with those exact references and current speed. It does not read game.mining, bind or modify original, dispatch the native callback again, cancel, clamp or mutate the inventory/world.
- The ordinary native body remains responsible for actor selection, native speed and harvest calculation, mod dispatch, progress and side effects. HEAD-before-trait is explicitly a new composition order and is not presented as historical multi-mod equivalence.
- Candidate transformation fixes the input map at 644 members. It removes both old adjacent context hooks, adds one native HEAD, appends one Pluto module, and adds two source files. Two existing manifests change; all other original bytes are retained. The inverse removes only those additions and restores the original hook bytes. Both Items-level speed owners and all consumer policy bytes remain unchanged.
- Exact source versus packaged accessor-prefix hashes are separate. No new registry, actor cache, TLS, projected component identity or borrowing stand-in is introduced.

## Qualifications and remaining evidence

The module is internal to the generated host crate, not a public authoring API. Mixins' default module visibility is pub(crate), while BorrowedFacts/read and their fields are pub(super), usable within Game's module tree. Storage is preparing an explicit-visibility documentation/manifest delta; it is not part of reviewed 6c0a3f64 and does not silently alter this cutoff.

Actual selected-plan acceptance, generated visibility/lifetimes, applied Game type-check and complete 646-member composition are still unproved. A narrow target probe does not replace the full combined plan. The full plan must also show the new HEAD does not bypass native dispatch or run alongside either legacy lexical context hook; any competing cancellation/replacement requires a fresh composition decision.

Finite scalar differential cases in this group do not prove actor-swapping, authoritative remote Haste/Broad frame matching, guest permission paths or Game state transitions. Those remain native integration requirements. In particular, the guest delayed NaN result is a branch difference to preserve and disclose, not a reason to normalize arithmetic in the adapter.

Only source reading and small Python renderer checks were performed for this review. No local native, Cargo metadata, Rust, Mixins plan/apply or production activation occurred.

Storage independently reviewed the twelve additional Rust cases against actual rules/math and found no serious source-level type or expectation mismatch. Its qualification is retained: the official division is unclamped, but the unchanged TConstruct conditional policy has an existing final speed.max(0.0). Neither review ran Rust.
