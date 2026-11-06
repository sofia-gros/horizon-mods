# Explicit ingredient presentation staging

Status: source-only candidate. Five Python source-boundary guards pass; they are not Rust behavior evidence. No Rust test, native plan, apply or host type-check has run for this addition. The nine API assertions and four real-native fixture assertions are prepared, not passed. The earlier Java/menu69c6 package proof excludes this module. No TConstruct production hook is changed or removed.

## Concrete caller and boundary

The legacy #941 fake-material Book::build return adapter both decorates Entry.stacks with material-bearing ItemStacks and removes ID-only placements. Pluto's existing Definition veto can supply the placement refusal when registered before book construction. Its Preview::show reads an existing entry and cannot create those ingredient alternatives. This candidate covers the remaining **presentation model** operation only. Other component-bearing crafting displays can use the same bounded mechanics; no material IDs, marker schema, output selection or eligibility rules appear in the shared API.

The public API is `pluto_inventory_api::recipe_book::presentation`. It is host-independent and adds no registry or dependencies. A client-lib caller uses the one existing API instance; the game layer continues using `horizon_client::pluto_inventory` rather than adding another bundled crate dependency.

The thin native `recipe_book::pluto_presentation::stage` adapter consumes an owned Book value, lends the capability synchronously, then returns the Book and callback result. It does not access Game, a live UI cache, a world, an actor or a resource manager. A consumer can use it on the Book value it already owns during construction. There is no new automatic hook, state field, callback registration, persistent index lease or provider activation.

## Operation and preservation

The capability exposes current entry facts and borrowed existing alternatives. Each Replacement names a current positional Address and supplies a complete owned ingredient-override map. Address checks index, recipe ID, display ID, ingredient count and supported display kind. It is not a complete native definition comparison, freshness token, world identity or proof of ownership. Identical values from a reload can satisfy it; the caller still owns recipe-policy validation and publication timing.

`replace` validates the entire batch before changing any entry. On recoverable validation rejection it returns every owned proposal and leaves the Book untouched. On success it moves the exact full native ItemStack values into Entry.stacks. It does not clone opaque component trees, edit stack fields, encode or decode. This preserves backing storage and runtime fields without treating semantic component facts as reconstruction or commit authority. The method does not require Stack: Clone.

Only Entry.stacks changes. Result, variants, display geometry, entry ordering, grouping, display IDs, placements and native item ordering remain unchanged. There is no knowledge grant, slot movement, packet, recipe generation or output/remainder authority. Showing an ingredient does not make a recipe craftable; consumers retain the appropriate placement and manual-matching policies.

Omitted entries stay unchanged. A specified empty map clears overrides and restores downstream native fallback. A specified ingredient with an empty alternatives vector remains explicitly empty. Duplicate entry or ingredient indices fail; indices address compact native requirements(), not shaped grid cells with holes. Alternative and override ordering are preserved, not sorted or deduplicated.

The initial capability accepts shaped/shapeless crafting displays only. Furnace requests, including clear, return UnsupportedFurnace: official624c Book::ghost uses its native ingredient fallback for furnaces instead of Entry.stacks. This avoids promising a ghost effect that the actual path does not provide. Unregistered display ID -1 is allowed on a newly constructed book; this operation does not register that display.

## Bounds and failure qualifications

A batch allows at most8,192 entries,1,024 ingredients in each addressed entry,65,536 overrides and65,536 total alternatives. Checked accumulation prevents count overflow. These are structural limits, not payload-byte limits: the caller already owns its stacks and remains responsible for their construction/storage cost. The library avoids amplifying arbitrary nested payloads by cloning them. Validation does not inspect or normalize component contents.

Recoverable validation errors occur before mutation. This is **not** an OOM, panic, destructor or process-abort rollback guarantee. Replacing old values runs their normal native drops. The entire user callback is not one transaction: two successive successful replace calls are two explicit batches, and failure of a later batch does not undo the earlier one. External side effects performed by the caller are outside this capability.

## Consumer migration remains separate

The original fake-material hook, its marker/material checks and its placement policy are retained. A future migration must validate its complete actual Recipe records, construct the desired full native stacks, call this staging operation, then publish the resulting Book at the correct existing lifecycle boundary. Invalid or unsupported requests require an explicit consumer fallback; there is no silent eligibility guess.

Do not use invalidate_definitions as a generic post-edit step on a live book: its forget_book operation discards that model. A rebuilt book must receive the decoration again, and any live Game view/ghost cache handling is a separate integration obligation. This candidate neither installs such a hook nor claims that UI, remote-client or LAN behavior is complete.

## Evidence and pending validation

Design sources are official Horizon624c4f6265c02296fbe8a80f029dd0d987fb94c0 `recipe_book.rs` (Entry, requirements/grid mapping, Book::ghost), existing Pluto preview/invalidation contracts and the immutable #941 consumer8c56bb52b6b77fae6115e894af9e369a5c3f9e08. The private fork was not accessed.

Prepared tests cover ordered/current positional assertions, duplicates/ranges, clear versus explicit empty, unsupported furnace, resource limits, full-native payload move identity, all-or-nothing validation rejection and preservation of unrelated fields. Native fixture execution requires exact official declarations and the actual horizon-items package, not a semantic ItemStack or Book stub. The exact-fragment/real-package fixture preparer is provided but has not been run. Its execution and the official packaged-module/type gate remain pending. No test claim follows merely from these prepared assertions.
