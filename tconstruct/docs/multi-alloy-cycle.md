# Smeltery multi-alloy cycle

The Smeltery now snapshots recipes matching its current fluid identities,
shuffles them with unbiased bounded draws, and tries every candidate once.
Each attempt rechecks temperature, input quantity, catalysts, known output and
remaining capacity against the preceding successful changes. Newly introduced
fluid identities become candidates next cycle; existing small quantities can
be increased and used by a later recipe in the current cycle.

This follows `MultiAlloyingModule` and `AlloyRecipe` in pinned Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. The source caches identity matches
until a new fluid appears; this port recomputes them each cycle against the
current world recipe/tag/config snapshot. Random sequences do not match Java's
process-wide `Collections.shuffle`, but Fisher-Yates preserves the same uniform
permutation distribution and avoids lexical recipe preference.

Foundries still do not alloy. The four-phase machine schedule and its single
fuel-cost charge are unchanged. Fuel preflight uses the same conservative cycle
planner without random draws. The complete candidate tank and cloned world RNG
are staged; the RNG is published only after the existing machine/fuel
`write_many` succeeds. Any invalid plan, overflow or invalid draw discards the
whole cycle. Actual rejected machine writes do not publish tank changes.

Tagged input fluids remain unsupported. A recipe potentially selecting any
such fluid suppresses the whole cycle, rather than selecting a different recipe
based on name or shuffle order. Unrelated tagged fluid volume remains excluded
from available capacity by the existing snapshot. This is deliberately stricter
than the old first-match behavior and does not claim tagged-fluid parity.

Tests cover independent recipes, all competing permutations, one execution per
recipe, deferred new identities, evolving amounts, catalysts, capacity,
temperature, tagged inputs, invalid randomness and the fuel/commit wiring.
Actual in-game alloy distributions, fuel use, save/reload and LAN remain
unverified; real-machine testing is outside the v0.5 completion condition.
