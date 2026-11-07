# Bounded station-ingredient parsing

`pluto_inventory_api::crafting::ingredients` adapts the native station parser's
read-only item registry and ordered result list. It contains no recipe marker,
JSON grammar, namespace convention, Forge interpretation, material rule or
component key. No production TConstruct callback is registered by this slice.

## Dispatch contract

Register once during startup, before station tables load. The registry has one
shared owner in the existing inventory API crate; there is no second host
registry, item catalog, tag cache, world snapshot or unregister operation.
Registration does not reparse tables already loaded.

Each callback receives the unchanged borrowed JSON value and a `RegistryView`
over the exact `Items` supplied to this invocation. `contains_item` queries the
current item map. `ordered_tag` borrows the current ordered-tag entry: None
means no entry in that registry, while Some(empty) is an explicitly empty entry.
Order and duplicates remain visible to the consumer; Pluto does not resolve
namespaces, fall back to another world's tags or infer members from membership
sets. The older advancement membership-only/overlay APIs cannot represent this
load-specific ordered view and are not repurposed as a second source of truth.

Callbacks return Unsupported or Handled(ordered IDs). Unsupported from all
callbacks preserves the native parser and its complete ingredient structure.
Handled(empty) is an explicit no-match and does not fall back. More than one
claim, even byte-identical or empty claims, is a Conflict. Callback order is
lexical registration ID order; all callbacks run outside the registry lock,
even after a conflict is established. An insertion during dispatch affects the
next snapshot only. Empty and duplicate registration IDs are rejected without
replacing another callback.

A single handled list is validated before constructing a native ingredient:
- At most 65,536 entries, each at most 1,024 UTF-8 bytes, and at most 4 MiB of
  total ID bytes.
- Every ID must exist in the supplied current item registry.
- IDs must be unique. Duplicates are rejected, not silently reordered/deduplicated.

These are explicit service limits on returned proposals, not a sandbox for a
callback's internal allocations or JSON parsing. Consumer code remains
responsible for its own parsing depth, input-size bounds and deterministic,
read-only behavior. Syntax/namespace rules remain consumer policy. Invalid
proposals and conflicts fail closed as the native empty ingredient; the public
outcome keeps the reason and owner IDs distinct. The native hook does not add
logging, which could otherwise amplify per-recipe load errors.

## Native meaning and limits

The shared adapter owns `stations::ingredient` through one cancellable HEAD.
No providers returns Continue before callback/context construction; all
Unsupported preserves the original `Ingredient::parse(...).unwrap_or_else(...)`.
A valid handled list produces `Ingredient::of` with exactly that order and set.
This is a flat item-ID ingredient, not a component matcher or nested ingredient
AST. A provider must not use this result to represent rules requiring later
component checks. Such rules need a different explicit contract.

Official `StationRecipes::load` uses this helper for stonecutting and smithing
fields. The request therefore does not invent a station type, recipe ID or
caller identity. A consumer selects its explicit JSON envelope; it must not
assume every invocation belongs to stonecutting. The adapter does not modify
recipe results, inventory, recipe knowledge, displays or persistent ownership.

The complete-plan guard reserves this native target until additional
composition is proven. A competing HEAD could skip dispatch and a later rewrite
could resurrect a conflict/invalid ingredient. The native function fingerprint,
pristine source file, bindings/phase/priority and helper bytes are checked.
Remove old consumer hooks when adopting this API; they do not compose by default.

## Consumer policy remains separate

The test-only TConstruct callback retains the source
`tconstruct:stonecutting_ingredient` envelope, accepted object shapes, explicit
ID grammar, Forge difference semantics, union order/deduplication, depth bound
and empty/missing-tag policy. It is compared with the exact old helper from MOD
`8c56bb52b6b77fae6115e894af9e369a5c3f9e08`. These rules are not library behavior.
Tests use real native Items catalogs and exact official parser bodies, with a
modeled HEAD wrapper and unrelated custom-ingredient endpoints. They are not a
full station loader, gameplay, multiplayer or production activation test.
