# Dense ordinary armor wear

Partial high-priority #379 durability implementation, stacked on Tanned armor
wear #925 and retained armor writeback #923.

Admits Dense's complete canonical definition in the scoped stock armor path.
Positive material and saved Dense levels merge once; malformed/negative values,
unknown active modifiers and changed definitions fail closed. Dense runs after
Tanned (priorities 200 then 100) at the existing pre-native call boundary, before
native item-damage callbacks and advancement reporting. The final loss is
recorded once and committed through #923.

The probability is the source postfix sequence `1 - (1 / (1.5 ^ level))`.
The exponent and base are float operands widened for the power; its result is
narrowed to float before float division and subtraction. This deliberately does
not use the older shared `powi(-level)` approximation. Each remaining damage
point is kept when a world RNG float is greater than or equal to the probability.
Zero points, absent Dense and full prevention consume no draws. Tanned's clamp
or secondary-cause cancellation happens before Dense consumes any randomness.
Record and commit eligibility checks consume none.

This uses the authoritative world's existing RNG. It does not claim source
Java global-RNG stream identity, exhaustive power rounding identity on every
platform, or change the old shared Dense implementation for unrelated paths.
Actual save/reload, damage, dedicated/guest/LAN and prediction remain unverified.

All #923/#925 guards remain: stock correct-slot armor, current tags, intact
single-item records, no native enchantments or unbreakability, no positive stored
Overslime, and only the supported active modifiers. Reinforced, Unbreakable,
Solar Powered, Overslime, Barkskin and other prevention hooks remain outside
this slice. No source break animation or generic held-wear implementation is
added; native batched health-event ordering is unchanged.

Source: TConstruct f4e77d43f39ba1047bf93ae816b7e1beaab514c2 (MIT), `dense.json`,
`ReduceToolDamageModule.reduceDamage` and `PostFixOperator.POWER/DIVIDE/SUBTRACT`.
The retained canonical definition carries its source copyright notice. No native
extension or provenance-blocked fluid implementation is included. Base-chain
native signed-protection 09f0179db and upper-bound 5e47547e2 remain dependencies.

Brief static source/API and whitespace review only. Three pure regression cases
added but not run; no tests, build, planner, CI inspection or merge for this slice.
The separate batch integration pass owns executable verification.
