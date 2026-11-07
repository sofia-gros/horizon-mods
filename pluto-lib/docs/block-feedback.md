# Read-only block-event facade (development)

`pluto_inventory_api::blocking` is the host-independent consumer contract for
final raised-item feedback. It uses no Game, native item, native damage-source,
owned native state or equipment mutation type. Its one registry lives in the
existing shared API crate; the private native bridge only captures and converts
facts before dispatch. No duplicate native callback registry remains.

This slice builds on the guarded native seam at `ee6a05e0`. That seam and the
older equipment proof are separate cutoffs; this facade still needs combined
package/host validation. It does not repair or migrate TConstruct shield wear.

## Borrowed facts and lifecycle

A callback receives `&blocking::Request` synchronously. The request contains:

- Exact outcome flags and blocked amount, without filtering or normalization.
- Source kind, causing/direct numbers, attacker and projectile facts.
- Current actor number, dimension, creative and spectator facts.
- A read-only `equipment::Equipment` capability for head, chest, legs, feet, main hand,
  off hand and the currently active use slot.

Entity numbers are opaque values scoped to this current actor/world event, not
persistent identities or permission to look up another entity. Zero source
sentinels are preserved. `hurt == false` does not imply no blocked damage: hurt
cooldown can reject health loss after blocking was calculated. Zero, canceled,
sound-only and nonfinite raw outcomes remain visible for consumer interpretation.

Shared `equipment::{Actor, EquipmentSlot, Slot, ActiveUse, Equipment}` types
serve block and other observation APIs; `blocking` re-exports the same types.
Equipment `Slot` distinguishes unavailable, known empty, and item facts.
`active_use() == None` means no active use. `Some` with `slot == None` means an
active inventory slot outside the six equipment slots; its item facts can still
be known, empty or unavailable. The main hand reads the selected inventory slot,
not the host's potentially off-hand `held()` accessor.

Facts are captured at final block feedback, before native wear. They are not the
earlier protection snapshot and are not live post-wear inventory. Borrow them
only during the callback. The API intentionally has no write, wear, cancellation,
replacement, commit, entity lookup or deferred-dispatch method.

## One semantic component representation

Equipment reuses `advancements::predicates::ItemSnapshot`, `ComponentValue` and
`ComponentText`. No second component tree or UTF-16 representation is introduced.
The new `pluto_component_view` native support module owns the single NBT-to-DTO
conversion; both the predicate and block adapters call it. Integer widths,
arrays, floating-point bits and even invalid UTF-16 sequences remain unchanged.
The predicate adapter retains its existing availability and fallback behavior.

Block equipment facts use the native saved-component-patch encoder. A known
default patch is represented by an empty compound. This does not expand item
prototype defaults. Retained opaque wire backing anywhere in the stack tree, including a stale
patch, makes component facts unavailable instead of claiming an empty/complete patch.
The adapter uses the one shared `pluto_stack_storage::contains_retained_wire`
query from dependency `29935573` (central integration `4ac667fa`), rather than
copying child traversal. Other fields report the native semantic item view.

These DTOs are read/predicate facts, **not complete native stack identity or
commit tokens**. They must not validate replacement, wear or equality of opaque
wire-backed items. Native full-snapshot validation and mutation belong to the
separate transaction boundary, which is not implemented by this facade.

## Registry and native boundary

Register during consumer setup before enabling associated item behavior.
Namespaced IDs are unique; duplicate registration fails without replacement.
Explicit removal affects later dispatches, not the already selected callback
snapshot. Callbacks run in lexical ID order after the registry lock is released.
Only callback function pointers and IDs are retained, never an actor/world.
Poisoned registry state falls back to native feedback; write APIs report it.
Callback panics are not swallowed.

When neither protection nor block consumers exist, both copied-caller HEAD
adapters continue the unchanged official body. The block bridge also checks for
consumers before component conversion, and excludes remote, client-side, replay
and absent actors. Host guests use the actor already selected by the native turn.

Dispatch remains immediately before native `after_block`, inside the original
`if raised` branches of ordinary hits and explosions. Ordinary default knockback
is still afterward. Explosion velocity/push and `forget_noted` remain before it.
Native item wear, sound and shield-disable feedback still execute afterward.
This facade changes no copied caller or reviewed source hunk; the complete-plan
and source-regeneration guards still apply to the combined bundle.

## Integration and evidence

The existing inventory API exports `blocking` and `equipment`. One public native
module named `pluto_component_view` is inserted into client_lib.rs. The private
Game module `pluto_equipment_view::Snapshot::capture(&Game)` implements the same
read-only equipment trait for reuse by other native observation adapters. No new crate,
crate dependency, native state, registry copy, TConstruct formula or consumer migration
is required. The coordinator owns unified package/facade integration.

Four pure API tests cover raw facts/ordering, registration, slot distinctions,
and unlocked removal. Two native conversion tests cover float/UTF-16 fidelity
and known default patches. They are written but not run in this slice. The old
native-Game callback fixture is superseded by these contract tests. No native
apply/build or gameplay parity is claimed for this new facade yet.
