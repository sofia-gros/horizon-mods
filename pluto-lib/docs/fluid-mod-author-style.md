# Fluid API usage with Fabric-like registration

**Authoring direction and source-matched example, not a new runtime API.**
PlutoLib should let a mod initialize namespaced content and register typed
callbacks without repeating host-sensitive Mixins. This is a familiar usage
pattern, not Java syntax, Fabric binary compatibility, or identical game rules.
PlutoLib is optional: Horizon → Mixins → a mod, with PlutoLib between Mixins and
mods choosing its services. TConstruct chooses PlutoLib.

## What the reference contributes

The Fabric project's [events guide](https://docs.fabricmc.net/develop/events/)
describes event instances with typed callbacks, registration and documented
invocation/return behavior. Its [item guide](https://docs.fabricmc.net/develop/items/first-item)
organizes namespaced content registration under initialization. Its
[fluid guide](https://docs.fabricmc.net/develop/fluids/first-fluid) separates fluid,
block, bucket and rendering concerns. These pages were read on **2026-11-06**.
Only these authoring concepts are adopted here. In particular, the fluid guide's
entity-effect examples are not authority or timing contracts for Horizon.

## Existing API: one mod-owned initialization function

[The complete Rust example](../experiments/fluid-author-api/example.rs) uses only
existing public symbols at MOD source `8715b1232c0fca133fe012f35830e2108e653d59`,
published equivalent `89744814b8d536a117e82f8ea3c8cdc9798faa95`:

```rust
use pluto_api::fluid::{self, Definition};

pub fn initialize_fluids() -> Result<(), String> {
    fluid::register_all([Definition {
        id: "meadow:rainwater".into(),
        block: "meadow:rainwater_block".into(),
        bucket: "meadow:rainwater_bucket".into(),
        texture: "meadow:block/rainwater_still".into(),
        tick_delay: 5,
        drop_off: 1,
        light: 0,
    }])
}
```

This is already a small, namespaced registration surface. Identical fluid
registrations are idempotent; conflicting IDs reject the whole fluid batch.
That transaction does not install assets, register a native block/bucket item,
register another service, or make the example playable. The consumer supplies
content assets and approved startup wiring on every participating side. Do not
copy the registry into the consumer or silently ignore registration errors.

No automatic `ModInitializer`, universal event bus, builder or new facade is
introduced by this example. The current package has two dependency layers;
`pluto_api` and the documented inventory/native adapter paths share their actual
owners. A later ergonomic facade must re-export/delegate to those owners, never
create another registry or an unsupported reverse dependency. See the
[shared boundary](shared-api-boundary.md) and [installation contract](native-installation.md).

## A query is not an event

The example also accepts a caller-supplied read function and calls the existing
`contact::query_box_with(bounds, limits, sample)`. It returns typed contacts and
query errors without borrowing Game or selecting an actor. Incomplete results
stay unknown in the example's consumer decision; unloaded cells are not dry.
The sample closure is a synchronous read dependency, **not an event listener**.
The caller still owns the actual read scope, adjusted bounds and when to query.

The existing fluid API does not expose a multi-listener fluid-contact event.
Writing an attractive `EVENT.register(...)` spelling cannot create the missing
native contact phase or actor continuation contract. Do not show that spelling
as an available API, make queries dispatch effects, use a post-tick queue as
immediate contact, or infer mutable authority from an entity number.

For a future shared event, the intended authoring shape is: select a named event,
register a unique mod-owned listener ID plus a typed function during documented
initialization, and receive only the facts/capability valid at that event's phase.
The exact exported name is not chosen here. Before implementation, its contract
must state side/phase, observation versus decision versus mutation, lifetime,
listener order and aggregation, duplicate handling, fallback, reentrancy and
error/panic behavior. A generic Pass/Cancel enum must not silently replace the
existing service's semantics. Native mutation can only be offered after its
same-call owner and continuation guarantees exist. Read-only and mutation APIs
must remain visibly distinct. See the [contact validation gates](immediate-contact-validation-plan.md).

## Render source is a separate, unverified cutoff

The newer render group is source-only local
`510f0fa89551f89c81097052ced06235037a6451`, published
[`a432ce4561ae1c81448b37a5b19e71126d8c4e72`](https://github.com/sofia-gros/horizon-mods/commit/a432ce4561ae1c81448b37a5b19e71126d8c4e72).
It adds `RenderDefinition`, `RenderOrientation`, `register_all_with_render` and
material-content helpers without replacing the fluid registry. The old example
uses no new render symbol and does not claim that cutoff passed native checks.

That newer path requires definition/render registration before standalone-model
preparation, which occurs before the old `native_blocks` content boundary.
A common initializer abstraction must preserve this real order and client-only
render preparation, rather than merely rename the old hook. Top orientation is
display geometry, not proof of upward flow or inverted contact shapes. Its
lower host-qualified provider paths are not a finalized mod-author facade.

## Verification boundary

The example is expected to type-check against the named existing API; **it has
not been compiled or run**. The small Python source checks pin the existing
public signatures, example fields and both independent source cutoffs. They do
not substitute for an actual API compile or host proof. No production source,
manifest, callback registration, CI workflow or consumer activation changes.
A future authorized check should compile this exact file against the real
`pluto-api` crate, not a copied or mocked Definition/Contacts implementation.

Source checks: **4/4 passed in 0.022 seconds**. The first attempt was 3/4
because a test expected different wording for the same-registry statement;
only that assertion was corrected. Neither attempt compiled or executed Rust.
