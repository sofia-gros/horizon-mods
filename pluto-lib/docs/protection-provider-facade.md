# Host-independent protection providers

This Step 2 defense-group slice removes native player/context/policy types from
the provider callback. PlutoLib owns one registry in the existing bundled
`pluto-inventory-api` crate, reached through
`horizon_client::pluto_inventory::protection`. It does not add a bundled crate,
copy a registry into another host package, change the official source cutoff,
or migrate any TConstruct consumer.

## Provider capability

A provider is `fn(&Request<'_>, &mut dyn Contribution)`. Register a namespaced
ID during setup. Invalid or duplicate IDs return an error; duplicates never
replace another provider. Removal affects later callback-list snapshots.

Native adapters bind each callback's contribution capability to that callback's
registration ID. `set_lower`, `set_upper`, `set_additive` and `clear` delegate to
the existing native `Policy` under that one ID. The callback cannot supply an
arbitrary ID, inspect another provider's contribution, change the victim or
commit inventory through this interface. This is a narrow API contract, not a
security sandbox for arbitrary native mods.

Validation and all arithmetic still have one owner, the existing native Policy:
finite lower <= 0, finite upper in 20..=25, signed finite additive points,
replacement rather than accumulation on repeated writes, stable lexicographic
addition before clamping, independent bounds, and unchanged overflow fallback.
An invalid setter preserves its prior value. Clearing removes only the current
provider's bounds/additive term. No new damage formula is introduced.

The registry copies IDs/function pointers in sorted order, releases its lock,
then visits providers. A callback may change registration without deadlock;
that change applies to future snapshots. Registration poisoning disables
providers. Panics propagate through the existing RAII hit scope. Provider
invocation must remain pure policy calculation, not external side effects or
another damage operation. No-provider paths continue natively before scope,
Blocking clone or semantic equipment conversion.

This intentionally replaces the earlier development `health::pluto_protection`
register/remove API, which exposed native types and silently replaced duplicate
IDs. The old `HitContext` module is removed; no legacy provider shim/second
registry is retained. Native helper entry points remain adapter internals.

## Fact provenance

- Victim entity number and client-side, replay, spectator and invulnerable flags
  are actual motion facts copied at provider dispatch. Invulnerability is not
  labeled creative mode. Invocation precedes native hit gates and is not proof
  of a successful or authoritative injury.
- Optional dimension and six-slot equipment are projected from the earlier
  captured native snapshot, never reread from a live Game. Unadapted paths keep
  None. Shared `ItemFacts` handles semantic component conversion, including
  unavailable opaque retained/nested wire data. No second component schema or
  converter is introduced.
- The additive `equipment::EquipmentSlots` trait exposes only the shared slot
  query. Existing `Equipment` implementations have a blanket projection. The
  protection snapshot never captured active-use state, so this capability has
  no active-use query instead of falsely returning inactive.
- `Source` borrows the actual hit's resident catalog through the existing native
  source view. Name/type/tag queries delegate directly. The public four-variant
  bypass enum maps one-to-one to native bypass variants, not hard-coded tag
  names. Unknown IDs yield None; missing tags on known IDs yield Some(false).
  No native catalog ID, definition struct, world lookup or cached name escapes.
- Requested amount preserves raw bits before native difficulty/block/cooldown
  and reductions. The living-non-player flag is the caller's actual value;
  causing entity's zero sentinel becomes None. No direct entity or weapon is
  invented.

All request references and contribution capabilities are synchronous borrows.
They are not persistent actor identity, a live inventory view or a write token.
The current-motion and earlier-captured facts are deliberately labeled rather
than asserted to be one post-hook snapshot.

## Native integration

The `run` adapter creates source/equipment views, visits the one API registry,
then installs the composed native Policy into the same scoped protection flow.
Two private modules implement facts and provider-scoped contributions. Existing
native caller copies, hook phases, source fingerprints, guard requirements and
protection arithmetic are unchanged. The shared component converter requires
facade `5785d0c3` and retained-wire query `29935573`/central `4ac667fa`; additive
math follows `f48dff45`/central `a27fe647`.

This follow-up is outside the previous combined native proof `999468ef`, which
validated the corrected armor observation and additive protection before this
facade. Existing native proof must not be relabeled as this callback migration.

## Focused evidence

Four host-independent protection API tests and one EquipmentSlots projection
test pass. They cover raw/unknown facts, absent equipment, no-provider behavior,
lexical ordering, duplicate/invalid IDs and unlocked snapshot iteration. A
separate source fixture compiles the actual Policy implementation and scoped
capability against the actual facade; its test proves own-ID updates, invalid
write preservation and clear leaving another provider intact. This fixture is
not a native Horizon build or game test.

Two native facts tests are written for known/missing/unknown source semantics
and captured main/offhand equipment. Their execution, full official package
plan/apply, client lib/bin check and combined compatibility remain pending the
next coordinated cutoff. Native test filter: `pluto_provider` (three tests,
including the capability fixture as a native module test). Public API filter:
`protection::tests`; the shared projection test is
`pluto_equipment_slots_projection_does_not_query_active_use`.
