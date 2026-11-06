# Borrowed damage source context (development)

The protection provider receives the host-independent
`pluto_inventory::protection::Request` and per-registration `Contribution`
capability. It no longer receives native PlayerMotion, HitContext, Policy or
DamageTypeId types. See [the provider facade](protection-provider-facade.md).
The underlying resident source adapter remains private to Pluto's native path.
The callback phase and native hit inputs are unchanged.

## Resident source facts

`request.source` is a borrowed `protection::Source` trait object. Name, type/tag
membership and the four armor/effects/resistance/enchantment bypass queries
use the actual `HealthResources.damage` catalog and native source ID. The
private adapter delegates through `damage_source::pluto_context::SourceView`
to official DamageSource methods. It does not recreate tag-name tables,
cache catalog results or invent damage formulas.

An invalid source ID yields `None` for name/classification queries.
A valid source absent from a tag, including a missing tag, yields `Some(false)`.
This distinction never cancels or accepts the hit: native client-side guards
and native unknown-source error handling remain authoritative and in place.
The lower-level native `SourceView::new` remains an adapter utility requiring its
actual catalog/id pair, not the supported consumer callback interface. A DamageTypeId is only a catalog-local index; the caller must
supply the correct catalog. The same index may name a different type in another
world. There is deliberately no ID-to-name cache or cross-world lookup.

The native player's DamageSource has no direct/causing entity stored in it.
The separate caller inputs are available without guessing:

- `requested_amount`: the exact native method argument before its difficulty,
  blocking, cooldown and reduction stages. It is not damage taken or damage
  after resistance. No float normalization or clamping occurs here.
- `living_non_player_cause`: the caller's native difficulty-scaling flag.
- `causing_entity`: the native numeric context ID, with sentinel zero as None.
  This is not a resolved entity reference and does not imply its kind.
- `equipment`: the optional read-only shared six-slot semantic view of the existing
  [equipment adapters](damage-equipment-context.md). Unadapted hits remain None.

No direct entity, source position, weapon, team, actor relationship or effective
scaled damage is invented. Consumers retain responsibility for interpreting
these generic facts and computing their own policy. The context and catalog
are borrowed for the synchronous callback; provider registration and native
hit evaluation keep the existing ordering and lifetime rules.

## Candidate validation

Four focused world-layer tests cover native query parity, unknown IDs, missing
tags and independent catalogs reusing the same numeric index. They passed in
the previous combined native cutoff `999468ef`. The new host-independent
projection and callback facade are outside that proof and await the next grouped
apply/check, including their new native adapter tests. This page does not claim gameplay integration or
recovery of existing TConstruct PRs.
