# Write a small recipe extension

PlutoLib aims for the familiar API style of typed registration and small
callbacks: import the operation, build its inputs, then handle its explicit
result. This is a Rust API design goal, not Fabric's Java API, loader, registry
ABI or a promise that every Fabric event has an equivalent.

**Status:** this additive fluent syntax and the two complete examples are a
new, source-reviewed candidate. Their Rust assertions and example compilation
have not run. The earlier 134-test proof applies to source `9da10132`, without
these methods. The native Book adapter and production consumer activation also
remain separate, unverified gates.

## Two layers, one implementation

The Mod-facing layer provides the fluent `Replacement` constructors and small
usage examples. The underlying layer keeps explicit data records, validation
results, ordered batches and the scoped native adapter. The fluent methods only
construct those same records; they do not duplicate validation, registries or
state. Existing direct-record callers remain compatible.

These are API layers within the current architecture, not two new crates or
runtime instances. The current host-independent dependency direction and thin
native boundary remain unchanged. The internal records are still public for
existing consumers; no visibility break or silent legacy migration is included.

## Stage ingredient presentation with owned values

```rust
use pluto_inventory_api::recipe_book::presentation::{Error, Replacement};

// `book` is the scoped Presentation capability, not a live Game or registry.
let entry = book.entry(entry_index).ok_or(Error::MissingEntry(entry_index))?;
let proposal = Replacement::for_entry(entry)
    .with_ingredient(0, material_bearing_stacks);
book.replace(vec![proposal]).map_err(|rejected| rejected.error)?;
```

The [complete generic example](../examples/recipes/presentation.rs) includes
all imports and a function signature. It accepts the consumer's full stack
values and requires no `Clone`. The builder copies the short entry address,
including its recipe ID string, and moves supplied vectors. It never rebuilds a
stack from a semantic component view.

`for_entry` starts with an empty override map. Applying it without any
`with_ingredient` calls **clears** that entry. An explicitly empty ingredient
vector is different: it creates an empty alternative list for that ingredient.
Appending an ingredient twice does not use last-wins behavior; the unchanged
batch validator rejects it. Bounds are checked by `replace`, not by the builder,
and they do not bound the caller's earlier allocations or payload byte sizes.

This adds convenience methods to the existing public `Replacement` struct.
Direct struct literals and `Presentation::replace` remain supported with their
existing results and ordering. Neither method claims a persistent recipe token,
current-world ownership or placement authority. Furnace remains unsupported.
The complete example deliberately returns only the rejection's error and drops
its proposals; consumers that need those values must keep `Rejected` instead.

There is no new presentation event or global registration. The native adapter
consumes an owned Book and exposes this capability synchronously. The consumer
still owns entry selection, construction/publication timing and cache handling.
See the [full staging contract](recipe-book-presentation.md). No old TConstruct
hook is removed and no automatic lifecycle is installed by these examples.

## Register a bounded policy through the existing owner

The existing matching API already accepts a typed `Registration` with an ID,
a cheap interest function and a decision callback. The
[complete registration example](../examples/recipes/matching_registration.rs)
uses that API without adding another registry or facade:

```rust
matching::register(Registration {
    id: "example:component_result_policy",
    interest: |recipe| recipe.template.is_some_and(|value| value.has_components),
    callback: |_request| Decision::Deny,
})?;
```

This intentionally broad example belongs to its example mod. It is not a
TConstruct eligibility rule and is not automatically installed. The consumer
calls registration once from a supported startup phase before relevant use.
A duplicate ID is an error; it does not replace another provider. The existing
lexical callback order and deny-wins behavior are unchanged. `Pass` cannot grant
crafting or provide a result. Matching denial is not recipe-book placement
refusal: consumers must select the separate existing placement policy where
required.

These callbacks are higher-ranked Rust function pointers. Noncapturing
closures coerce to them; capturing closures do not. No retained world, actor or
per-screen state is added to make an example compile. Registration returns a
`Result`, and there is no new unregister/reload contract.

## Choose the actual phase and one API instance

- [Content Registry](api.md) is preparation-time declaration, not runtime
  registration. Its typed settings and `Result` remain distinct from events.
- [Short item use](item-use-b2.md) already uses
  `pluto_api::item::register(id, behavior)` and an `ItemBehavior` trait. This
  recipe change does not copy or unify its registry.
- Recipes use the existing `pluto_inventory_api` instance in client-lib. The
  game layer uses the existing `horizon_client::pluto_inventory` re-export.
  Do not copy source modules or introduce forbidden bundled-crate dependencies.
- Native adapters still have host-qualified paths. Fluent inputs improve the
  public API without pretending those lifecycle integration seams are solved.

The examples are concrete usage guidance, not simulated host tests. Later
checks must compile their unchanged source against the actual Cargo API crate;
source review alone does not establish Rust type compatibility or gameplay.
