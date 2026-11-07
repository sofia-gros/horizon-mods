# Additive protection contributions (development)

The provider facade accepts raw points through its per-registration
`Contribution::set_additive(points)` capability. The native hit `Policy` stores
that value under the registration ID through `set_additive(provider_id, points)`.
This is a generic numeric contribution API,
not an armor/material rule. Consuming mods decide eligibility and calculate
points using the borrowed hit/equipment context. No TConstruct formula, native
StatusFlags field, extra registry, new hook or persistent state is introduced.

## Composition contract

- Finite positive or negative points replace that key's previous additive
  contribution; they do not accumulate on repeated writes to the same key.
- Zero (including negative zero) removes only that key's additive term.
  Empty IDs and nonfinite values are rejected without changing the old term.
- Bounds and additive terms are independent. `set`, `set_lower` and `set_upper`
  only change bounds. `remove(provider_id)` removes both kinds for that key.
  `additive(provider_id)` reads its current optional term.
- Use a distinct namespaced key and only write your own contribution. Keys are
  caller-supplied identifiers, not a security boundary between mods.

At the existing native protection read, PlutoLib begins with the final native
**uncapped** raw value, then adds terms in lexicographic provider-ID order using
ordinary f32 arithmetic. Insertion/callback registration order does not change
that order. It then applies the composed lower/upper bounds. Native armor and
resistance have already run, and native effects/enchantment bypass and zero
checks remain outside this stage, in their original positions.

A negative additive term can reduce existing positive protection. It does not
implicitly permit negative total protection: the default lower bound remains
zero. For example, native raw10 plus additive-15 becomes zero under default
bounds, or -5 when a provider explicitly sets a lower bound at/below-5. A raw30
plus additive-15 becomes15, not5: the native20-point cap is applied after the
addition, not to the base first. The default upper bound remains20 unless an
independent bound contribution extends it, never beyond25.

Values within0..20 pass directly to the unchanged native scalar/gate. Values
outside that interval use the existing one-shot marker bridge to preserve the
post-resistance stage. A zero result is not resurrected as damage, and an
unrelated StatusFlags query is never changed. No additive terms, removal of all
terms, or an unchanged effective sum preserves the prior native/raw behavior.

## Invalid arithmetic and lifecycle

Nonfinite native raw input stays untouched. If any intermediate f32 addition
is nonfinite, the entire additive batch is discarded for that hit and the
original raw input is used. No partial prefix is retained, even if a later term
could cancel the overflow. Independently valid bounds still apply. This is
separate from the existing final-multiplication overflow fallback, which uses
the original native raw-EPF path rather than introducing nonfinite damage.

The native `set_current_additive` and current-bound functions are private
adapter protocol rather than the supported consumer interface. Consumers
register through the [provider facade](protection-provider-facade.md) and receive
a capability limited to their own registration. The
policy is hit-local; nested hits have independent policies and leaving the scope
releases it. No values are saved to actors, equipment, worlds or network state.

The existing complete-plan guards remain mandatory. In particular, other
numeric StatusFlags RETURN hooks must precede PlutoLib's projection, and the
private scalar-marker boundary cannot have other hooks. This slice does not
weaken those composition requirements.

## Focused evidence

Eleven host-independent tests in the exact protection-context source pass using
`rustc --test`, including five new additive tests. They cover replacement and
invalid writes, independent removal/bounds, stable order, uncapped native base,
negative-floor behavior, batch overflow fallback and scoped/one-shot reads.

Three native-kernel fixture tests were also added for armor/resistance ordering,
raw-before-cap/negative bounds, and bypass/zero behavior. They execute the
transformed official damage kernel when the grouped batch runs
`cargo test --locked --offline -p horizon-world --lib pluto_protection --
--test-threads=1` (18 tests total). Those 18 native tests and the combined package lib/bin check passed at
`999468ef`. The later provider-facade follow-up is outside that proof; neither
result is a live gameplay pass. No consumer PR recovery is included.
