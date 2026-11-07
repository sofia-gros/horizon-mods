# Player protection context (development)

This source-only PlutoLib service targets official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` and official Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. It adds no WASM dependency,
TConstruct dependency, native repository patch, host field, or native Drop impl.

## Responsibility

PlutoLib owns synchronous player-victim hit scoping, provider registration,
additive protection/bound composition, and the adapter to the native post-resistance
protection calculation. A consuming mod owns material rules, modifier levels, eligibility and numeric
formulas. The optional generic equipment handoff is described in
[damage equipment context](damage-equipment-context.md). The library does not
supply any TConstruct provider or claim to repair its existing state declarations.
This service is separate from outgoing player-weapon damage callbacks or shields.

## API

The host-independent provider API is `horizon_client::pluto_inventory::protection`,
re-exporting the one `pluto-inventory-api` registry. `register(id, callback)`
requires a namespaced ID and returns an error for invalid/duplicate IDs rather
than silently replacing a provider. `remove(id)` affects future callback-list
snapshots. This replaces the earlier development callback exposing native
PlayerMotion/HitContext/Policy types; no TConstruct consumers are migrated here.

```rust
fn policy(
    request: &horizon_client::pluto_inventory::protection::Request<'_>,
    contribution: &mut dyn horizon_client::pluto_inventory::protection::Contribution,
)
```

Each callback receives only read-only facts and a capability bound to its own
registration ID. It can set its own lower/upper bound and additive raw points,
or clear its own contribution. The capability cannot name another provider.
Native `Policy` remains the sole implementation of validation and composition:
finite lower <= 0, finite upper in 20..=25, signed finite additive values, default
bounds [0,20], and deterministic ID-ordered addition before bounds. Invalid
writes preserve prior values. See [additive protection](additive-protection.md).

Callbacks run in lexical provider-ID order outside the registration lock and
must only calculate policy. They cannot use this capability to mutate equipment,
cancel a hit, dispatch damage or commit inventory. Callback panics propagate;
RAII restores the native scope. Poisoned registration state disables providers.
With no registered provider the native wrapper continues immediately without
scope reentry, equipment conversion or a Blocking clone.

The request distinguishes current motion facts from captured inputs. Entity,
client-side/replay/spectator/invulnerable flags are copied at provider dispatch.
It does not infer creative mode. Optional dimension and six-slot equipment
come from the earlier exact Game capture, not a fresh live inventory read.
Bare/unadapted paths retain None. Equipment exposes the narrower shared
`EquipmentSlots` trait because active use was never captured; unknown active
use is not reported as inactive. Source queries borrow the actual hit catalog;
see [source context](damage-source-context.md). Providers still run before
native damage gates, including client/replay/unknown-source handling, and must
not interpret invocation itself as a successful or authoritative injury.

The native world `Policy`, thread scope and `set_current_*` functions remain
adapter implementation details, not the supported consumer registration API.
There is no duplicate registry and no native type in the callback signature.
All values and capabilities are synchronous borrows, not retained actor state.

## Native adapter and ordering

1. A maximum-priority cancellable HEAD on `PlayerMotion::hurt_caused_by` creates
   an RAII scope and invokes the same native method under a one-shot reentry
   token. The immediate inner entry consumes the token. Other HEAD hooks and the
   unchanged native body run once. Nested hits get separate scopes; errors,
   cancellation and unwinding drop the scope normally. The optional Blocking
   input is cloned once; the native method consumes that clone.
2. `Environment`'s existing `ArmorWorld::server_level` method arms the borrowed
   address of its own temporary `StatusFlags`. No pointer is dereferenced,
   allocated, or used as persistent identity. A matching protection read consumes
   that arm once; other status queries preserve their ordinary return values.
3. A RETURN hook on `StatusFlags::protection` sees the final value, including
   lower-priority consumers' cancellable HEAD returns. It adds the scoped named raw-point contributions before applying bounds.
   When the scoped policy needs a negative result or a cap above 20, it retains
   the bounded value and
   returns a private positive marker through the native `protection > 0` gate.
   Changed values within0..20 use the native gate/scalar directly.
4. The existing scalar `combat_rules::after_magic` consumes that marker once
   within the same thread and scope, calculating `damage * (1 - EPF / 25)`.
   Unmarked calls use the unchanged native implementation.

On this official source pair, the server-level lookup, protection read and
scalar dispatch are synchronous and contiguous with no intervening native
callback. Native effects/enchantment bypass gates, resistance, armor wear,
absorption, health, hunger, hit cooldowns and totems stay in their original code
and order. A bypass never calls the marked stage. Ordinary policy and nonfinite
protection values retain native behavior. If extended arithmetic is nonfinite,
the adapter falls back to the original raw-EPF/native-bounds result; it never
turns finite damage into an infinity solely because of extended bounds.

Run `python3 pluto-lib/tools/check_protection_plan.py COMPLETE_PLAN.json`
after planning every enabled bundle together. This rejects conflicting maximum
HEAD priority, hit RETURN hooks, late numeric status modifiers, any additional scalar
hooks (including RETURN hooks that could read the private marker argument) and a missing marker consumer. The check is required for composition;
an isolated library plan does not validate other installed mods.

The highest HEAD priority is reserved for this scope wrapper. Other integrations
must not install another equal-priority wrapper there. The official target
contains `?`, so the current Mixins validator rejects RETURN hooks on that
method; this avoids applying such hooks twice on reentry. The status projection
RETURN priority is -10000 so ordinary numeric consumers finish first. Changes
to either native control flow, Mixins RETURN support, or callback ordering need
fresh integration validation. The marker/arm functions are internal adapter
protocol, not consumer APIs.

## Validation and remaining work

The host-independent provider follow-up is outside every native proof below.
Four pure facade tests and one fixture using the actual native Policy source
and scoped capability pass. Two native source/equipment projection tests are
written but await the next grouped apply/build/test cutoff. See
[provider facade details](protection-provider-facade.md).


The isolated service was applied to pristine official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` by official Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. Runtime source commit
`78b9ea45de1a97fba408fb1fed26e8dacc0e4da6` passed:

- Official plan and application: four injections, two runtime modules, one
  test-only fixture module, and zero state declarations.
- The complete selected-service plan-ordering guard.
- `cargo check --locked --offline -p horizon-client-lib --lib`.
- `cargo test --locked --offline -p horizon-world --lib pluto_protection --
  --test-threads=1`: ten passed, 361 unrelated tests filtered out. Six tests cover
  scope/policy arithmetic, identity/one-shot consumption, nesting, unwind,
  thread isolation and overflow fallback. Four fixtures execute the transformed
  official damage kernel, covering armor/resistance ordering, signed and upper
  bounds, native defaults, effects/enchantment bypass, zero and error paths.
- Five plan-ordering tests and both native-package tests.

These are focused API proofs, not full PlutoLib/TConstruct integration or
end-to-end gameplay. Actual provider registration followed by a live native
player hit, equipment refresh, respawn, save/reload, LAN guests and prediction
still require integration tests. The native-kernel fixtures use a controlled
DamageReductionWorld environment, not a live Game or PlayerMotion fixture.

No existing TConstruct StatusFlags fields or missing min/max method targets
are migrated by this library change. Consumers remain blocked until ported
to supported owned data and this API, then checked as a combined candidate.
Keep TConstruct equipment rules and formulas in the consuming mod.

The earlier field-based experiment was rejected by the official planner because
StatusFlags is wholly public. It remains unpublished; this service does not
relax that check or change the public struct's construction/move semantics.
