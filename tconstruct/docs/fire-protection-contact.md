# Fire Protection standing-in-fire duration

Related to #379. Stacked on the explicit player ignition duration bridge.

The native `MotionWorld::apply_fire_contact` still performs collision lookup,
spectator clearing, water checks, thawing, contact damage and the native fire-clock
update. Only its local `multiplier` read for `ignite_contact` changes when the
existing Fire Protection context and the unmodified native attribute match.

The replacement multiplier makes the existing `ceil(160 * multiplier)` evaluate
to the source integer `160 - floor(160f * level * 0.15f)`. The normalized argument
uses a half-tick interior point to avoid an extra tick from division rounding.
It is never saved as an attribute or item component. The original negative-clock
warm-up, one native RNG draw for a nonnegative player clock, maximum with the
current fire time and invulnerability behavior all remain native.

This also repairs the preceding explicit-ignition injection: its whole-expression
selector contained `?`, which the injection compiler refuses. Both paths now use
supported MODIFY_VARIABLE reads. The explicit helper uses immutable player
access to avoid a nested mutable borrow in `set_remaining_fire_ticks`. Its guard
compares the already computed native duration to the expected baseline; custom
attributes that differ but round to the same tick are indistinguishable there.
The contact path compares its actual local native multiplier before substitution.

## Boundaries

The shared worn-armor level resolution, completed fractional enchantment levels,
source maximum across armor, broken-piece semantics and reload/tick/equipment
refresh are unchanged. No Fire Protection damage reduction, mob equipment bridge,
held-shield bonus or remote-client projection is added. Other direct fire-clock
writes and full legacy-versus-host contact behavior are not claimed. Native
contact clock increments are deliberately retained, including its RNG behavior.

Two further regression cases were added for all supported ordinary levels,
normalization, native-attribute mismatch and explicit-ignition fallback. They
were not run. Brief static review only; no tests, builds, planner runs, CI
inspection, version checks or merges. Runtime matching, save/reload and LAN
remain unverified. Integrate this branch with its prerequisite, not the prior
explicit-ignition commit alone.
