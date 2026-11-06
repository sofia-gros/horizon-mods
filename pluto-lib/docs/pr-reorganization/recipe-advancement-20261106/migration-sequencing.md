# Remaining recipe and advancement migration sequencing

This is a read-only source review, not a consumer implementation or a newly observed gameplay failure. The risk below concerns a naive future migration. Java/menu integration remains fixed at local `69c6b72b0b79b64317483bede6a2b2a689d5b83a` / [remote `7aaf04c6`](https://github.com/sofia-gros/horizon-mods/commit/7aaf04c6091b74a6d343137a26aa95069c9b7e7a); none of its source or pending Rust proof was changed.

## Recipe definitions and index publication

The old [fake-material refresh](https://github.com/sofia-gros/horizon-mods/blob/8c56bb52b6b77fae6115e894af9e369a5c3f9e08/tconstruct/mixins/tconstruct/source-fake-material-world.rs) first computes `generated(game).unwrap_or_default()`, mutates live crafting definitions, then replaces the recipe index, refreshes discovery, restores saved knowledge and invalidates UI.

The old [Travelers refresh](https://github.com/sofia-gros/horizon-mods/blob/a8fc1e18e9c819c93a60cd50aea87dc8a099efaa/tconstruct/mixins/tconstruct/source-travelers-world.rs) similarly mutates live crafting definitions and metadata before replacing the index and invalidating UI.

The shared [index capability](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/recipe_book/index.rs) is fallible: it validates the complete current index, exact expected values, removals, collisions, range and resource limits. Simply replacing the old final `replace` call with a fallible compare-and-apply can leave recipes changed while the index is unchanged.

For successful coherent publication, stage the affected crafting vector, metadata map where applicable, and index; validate candidates before a no-fail publication tail, then invalidate presentation. Official [RecipeIndex](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/recipe_book.rs#L693-L707) is Clone, whereas [Recipes](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/crafting.rs#L1327-L1340) has no blanket Clone implementation here: stage the actual affected fields rather than inventing a whole-world clone.

**Failure behavior needs a separate decision.** Both old refreshes remove stale synthetic recipes when source generation is unavailable or invalid. Changing every shared-index error to “return without modifying anything” can preserve stale crafting definitions and reverse that fail-closed policy. In particular, an invalid source snapshot and a rejected complete-index validation can occur together. The caller must define a safe failure path for crafting availability and stale presentation; current CAS alone does not provide that policy, persistent ownership or a multi-table transaction.

This is not a claim that every related advancement/knowledge/network mutation can be made atomic by staging these three collections. S09/S10 are deny-only capabilities, not output authority or arbitrary family ownership.

## Loaded advancement projection

The old [discovery world adapter](https://github.com/sofia-gros/horizon-mods/blob/8c56bb52b6b77fae6115e894af9e369a5c3f9e08/tconstruct/mixins/tconstruct/source-fake-material-discovery-world.rs) has both a normal tree-return hook and an explicit refresh for an already loaded tree. Its cached native AdvState field cannot simply be retained on the current official target.

The shared [tree adapter](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/advancement-projection-world.rs) returns `NotLoaded` for the native empty-tree sentinel and does not consume prefetch. Removing the old return hook without a correctly timed, existing synchronous retry can miss first-load projection. Keep readiness/retry policy in the consuming mod; do not add a global cache, force a load, reset progress or infer persistent ownership from equal values.

## Controller events and inventory metadata

- The existing controller [event policy](https://github.com/sofia-gros/horizon-mods/blob/8c56bb52b6b77fae6115e894af9e369a5c3f9e08/tconstruct/mixins/tconstruct/source-container-advancement.rs) maps only Smeltery/Foundry types. Keep this mapping local.
- Local events follow successful menu opening. Guest events must follow the validated opened-menu check **inside the active guest scope**, not after restoring the local actor. The shared [event adapter](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/advancement-events.rs) borrows the current actor; it does not select or cache one. `Submitted` does not mean an advancement was awarded.
- [Component inventory events](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/advancements.rs) remain default-off. Consumer registration must request them before the first relevant native inventory event. Lazy activation inside a predicate is insufficient because an unchanged native item/count hash can prevent that predicate call.

## Required consumer acceptance cases, not executed here

1. Valid ready snapshot and valid index: stage all affected recipe/index values, preserve unrelated records and publish before UI invalidation.
2. Unavailable/invalid source snapshot with valid index: preserve the old policy of removing stale synthetic recipes.
3. Unavailable source plus index validation/collision rejection: prove the chosen failure path cannot leave stale crafting enabled merely because the CAS failed. Exact fallback behavior must be specified before implementation.
4. Stale expected index caused by another owner: preserve unrelated values and regenerate/revalidate instead of overwriting or claiming a lease.
5. Not-loaded/prefetched advancement tree: do not force loading; demonstrate projection after the normal ready transition.
6. Local, authoritative guest, remote client and absent actor: keep native actor/progress rules and never duplicate an award by submitting both local and guest paths.
7. First component-only inventory change: startup registration is active before the native hash gate; library-only installation still follows the untouched native path.
8. Byte-identical reload: current-value equality is not evidence of the same world/index lifetime.

No Rust tests, native plan, host build, Actions operation, production activation or PR-state change was performed for this review. No discarded fork was accessed. These findings refine the existing B01/B02/B03 boundaries; they do not declare Step 2 globally complete.


## Follow-up: preserve book presentation as well as placement refusal

This follow-up is a source trace only. It does not change the frozen Java/menu integration, register a provider or run a host.

The original #941 [book return adapter](https://github.com/sofia-gros/horizon-mods/blob/8c56bb52b6b77fae6115e894af9e369a5c3f9e08/tconstruct/mixins/tconstruct/fake-material-book-build.rs) delegates to [decorate](https://github.com/sofia-gros/horizon-mods/blob/8c56bb52b6b77fae6115e894af9e369a5c3f9e08/tconstruct/mixins/tconstruct/source-fake-material-book.rs#L66). That operation has two separate effects:

1. It fills each validated synthetic entry's ingredient alternatives with full material-bearing ItemStack clones, so the visible ghost and overlay identify the required material.
2. It removes the entry's ID-only placement record, so mixed materials cannot make it appear craftable.

The second effect already has a generic route, **provided the consumer registers its veto before the book is built**. On official 624c, [Book::build](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/recipe_book.rs#L359) obtains placements through Recipes::placement. Pluto's Definition-phase veto returns None at that boundary. Book::can_craft then returns false for the absent placement, and the native [build_view](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/game/recipe_book.rs#L508) uses that result to exclude it only when the craftable filter is enabled. The unfiltered book remains a separate visibility path. Late registration cannot retroactively change an already cached Book; if late registration were ever permitted, explicit invalidation and a rebuild would also be required. This is not a request to permit late registration.

The first effect is **not replaced by Preview::show**. The current [preview adapter](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/recipe-book-preview.rs) captures and validates the existing entry, then calls native fill_ghost. Its pure ingredient_previews helper resolves an explicitly supplied override list but does not install overrides into native Book entries. Therefore removing the old book-return decoration merely because preview and placement-veto services exist would lose the material-bearing ingredient display. A future consumer port must preserve that verified decoration path or implement and prove an explicit generic presentation-update boundary. TConstruct's marker validation, material selection and recipe eligibility remain consumer policy.

Do not describe this as successful component-sensitive placement, automatic output authority, persistent index ownership or a demonstrated current gameplay bug. It is a concrete missing migration obligation in the legacy consumer. No new public helper has been added just to fill it.

Additional acceptance cases, not executed:
9. A correctly decorated component-sensitive recipe stays visible in the unfiltered book, has material-bearing ghost/overlay alternatives, and remains absent from the craftable-only view even when item-ID stock is sufficient.
10. Startup Definition veto precedes Book::build; rebuilding after explicit invalidation preserves unrelated entries and avoids retaining a stale placement cache.
11. Preview failure does not grant placement, and removing/replacing a recipe cannot reuse a stale selection to display different current entry data.

The source trace also reconfirmed that index display ordering is family policy: fake-material uses sorted append allocation, whereas Travelers fills free gaps and preserves its notification flags. The shared CAS supports explicit allocation policy; migration must not silently normalize these differences.
