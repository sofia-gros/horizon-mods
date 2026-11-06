# Ingredient presentation: native connection and bounded proof plan

This is a read-only execution plan, not a native test result or an activated
consumer. Source is frozen at `dd491b3a1e6d0431eeac6965c2d18ed9f0a1ea11`.
The earlier API134 proof tests `9da10132` without the later fluent builder.
The planned authoring CI tests the pure API and examples only. Neither result
substitutes for the four native fixture assertions below.

## Actual official path

At official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`:

- `game/recipe_book.rs:384–389`: `Game::ensure_book` creates an owned value with
  `Book::build(self.recipes(), self.items())`, assigns display IDs, then stores
  it in `recipe_ui.book`. This is the sole `Book::build` call found in that
  official client source tree.
- `recipe_book.rs:358–446`: `Book::build` constructs entries, placements and
  item order, returning an owned `Self`. It contains no shared presentation
  provider call. The old mod #941's `fake-material-book-build` uses a RETURN
  hook with owned `Book` plus borrowed `Recipes`.
- `recipe_book.rs:484–561`: crafting ghost construction reads `Entry.stacks`
  by compact requirement index, clones selected display values for rendering,
  and skips explicit empty alternatives. Furnace ignores that map and uses
  its native ingredient/fuel fallback. The new capability intentionally refuses
  Furnace rather than promising a visible change there.
- `forget_book` drops the cached model; `ensure_book` reconstructs it later.
  Any future decoration must be applied to each construction. Invalidating
  definitions after decoration would discard it, not make it persist.

The current Pluto adapter is already shaped for an owned result:
`stage(Book, callback) -> (Book, callback_result)`. It changes only ingredient
presentation. It does not own Game's cache, display ID allocation, knowledge,
placement refusal, recipe selection or the native build lifecycle.

A future consumer can decorate the returned owned value before cache
publication, but that is a **candidate connection**, not proof that current
Mixins can transform the specific RETURN target. The original hook and all
TConstruct marker/material rules are retained. There is no new hook or provider
activation in this source group. No generic registration mechanism is inferred
from the pure staging API. Do not connect it until actual transform/composition
and lifecycle evidence exists.

The immutable #941 source also removes selected placements while decorating.
That second behavior must remain covered by the separate pre-build definition
veto during migration. Showing the right material does not block mixed-material
ID-only placement by itself. Do not remove the old hook merely because the
presentation fixture passes.

## What the four native assertions establish

The supplied preparer copies the unchanged production adapter and complete
real inventory API. It selects byte-exact official Ingredient/Shape/Placement,
BookType/Category/Display/Entry/layout/Book declarations, then depends on the
actual official `horizon-items` and `horizon-world` packages. No ItemStack or Book
semantic stand-in is permitted. This is a focused declaration-and-adapter
fixture, not the entire official Book implementation or live Game path.

The four cases cover:

1. Full native ItemStack backing is moved, with the same boxed-data pointer,
   while unrelated Book fields stay intact. This case uses the fluent builder.
2. A bad last request leaves both entries unchanged and returns both proposals.
3. Empty batch, omitted entry and explicit clear remain distinct.
4. Furnace refusal leaves the old presentation untouched.

API unit tests separately cover duplicate/range/budget/address cases. The native
fixture does not exercise `Book::build`, `Book::ghost`, packets, rendering,
resource reload, a real screen, LAN, matching, output derivation or placement.
It cannot by itself establish whole-host module installation/type compatibility.

## Execution phases and refusal conditions

No phase below has been run for the native fixture. Each requires a separately
approved serial slot and immutable inputs.

1. **Prepare exact source and resolve only metadata.** Materialize a small,
   complete source root from the fixed base plus the published presentation
   delta, not an incomplete patch directory. Reject symlinks and extra files,
   including extra Cargo configuration. Copy the complete real API; do not
   duplicate individual modules or pretend a different path is a cached crate.
   Require the clean official checkout/origin/revision, unique byte selectors,
   exact file hashes and actual local Cargo package paths. Resolve offline from
   the official lock, then reject every dependency identity outside that lock
   and the two named local packages. Freeze the resulting lock and file set.
   A matching lock/fingerprint is not proof that dependencies will be Fresh.
2. **Run only the four fixture tests.** Use `--lib --locked --offline`, jobs1,
   debug0 and incremental0. A reviewed compiler wrapper may admit at most one
   actual API library compilation and exactly one fixture `--test` compilation.
   It must reject any official/registry dependency regeneration before execution.
   Assert one exact four-passed/zero-failed/zero-ignored summary. Seal original
   source, actual copied sources, native fragments and locks before and after,
   including on failure. Use the continuous 10 GiB + 128 MiB floor, a bounded
   deadline and owned-process-group teardown. Do not automatically retry or
   progress to a full client build on failure.
3. **Plan the actual packaged module separately.** Use the pinned official
   Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`, complete package and all
   mandatory existing guards. A plan result is not apply/type-check evidence.
4. **Later integration gates.** Full packaged apply and host type-check remain
   separate. A consumer hook requires its own exact target/ordering/composition
   proof, repeated construction/reload behavior and visible ghost/overlay tests.
   Production TConstruct migration is not part of these four fixture tests.

## Current capacity decision

Read-only inspection found only about 11.3 MB above the normal disk floor.
A fresh real API library has previously required approximately 5.5 MB including
its metadata; a comparable completed native item fixture executable was about
7.4 MB. Those outputs alone exceed the remaining margin, before link/object
scratch space. Existing native libraries are present, but their presence is
not a guarantee of the exact Cargo feature/path fingerprint needed here.

Therefore the current safe decision is **do not launch the local native fixture**.
A conservative initial planning allowance is at least 24 MiB above the normal
floor, subject to actual dependency/output inspection and the continuous guard;
this is a budget estimate, not a measured upper bound. A failure-closed wrapper
must stop before any unexpected dependency compile. Alternatively, a separately
authorized native CI job could provide fresh capacity. The authoring-only CI
must not silently grow into that native job.

No cache or source deletion is requested by this plan. Only separately approved,
fully preserved completed test executables may be considered for recovery.
No fork, upstream edit, Actions dispatch, native compilation, Cargo metadata or
new stage was accessed or performed during this investigation.

## Remaining concrete consumer boundary, source review follow-up

The public display address is deliberately too small to decide whether a
material-bearing recipe is valid. The old `display_stacks` consumer checks the
complete shaped recipe: width/height/cell count, actual item IDs, custom marker
map and child shape, result count and its Material component, and the exact
generated recipe ID. Those checks stay in TConstruct. The existing generic
`Ingredient` trait exposes ordered IDs, custom data and child facts, but the
matching API's semantic `TemplateFacts` is not a full-stack construction or
commit capability. Do not reconstruct native stacks from it merely to eliminate
a host-qualified import.

The bounded migration candidate is therefore a thin consumer-specific native
adapter: keep its full Recipe validation and full ItemStack construction, borrow
that policy context synchronously, and delegate only ordered Entry.stacks
replacement through the shared `stage` capability. A new global display event
is not necessary for this first caller and has not been implemented. If one is
introduced later, it needs its own single registry owner, explicit native
context/stack transfer contract, provider ordering and same-entry conflict
policy; the current explicit batch API does not establish those semantics.

Concrete acceptance cases for that later consumer migration are:

- Keep malformed/nonmatching recipe skipping. Adding an unsupported unrelated
  Furnace entry must not reject every valid crafting decoration in the book.
- Keep every display of a matching recipe. Multiple entries sharing a recipe ID
  must remain separately addressed by index; do not deduplicate them by ID.
- Keep full material-bearing native stacks and the compact requirement order,
  including shaped holes. No semantic DTO reconstruction or source-key parsing
  moves into PlutoLib.
- Use the fresh owned build scope even while display IDs are still `-1`;
  `set_display_ids` runs afterward. An equal address from another build is not
  a persistent token.
- Pair the presentation migration with the already separate startup definition
  veto so the old placement-removal behavior is not lost. The builder itself
  never grants/refuses placement or changes the craftable filter.
- Reapply after `forget_book`/rebuild, without clearing the just-decorated model.
  Refreshing a live view/ghost is a separate Game lifecycle task, not implied by
  returning an owned Book.

These are source-derived requirements, not passed tests. The old consumer hook
and all production paths remain unchanged. The pure authoring CI can establish
builder compatibility and example type correctness, but not any of these
native/context/lifecycle cases.
