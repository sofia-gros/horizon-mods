# Blast Protection explosion knockback

Related to #379. Stacked on the protection acquisition and Fire Protection
secondary-effect chain, reusing its exact LevelingInt progress conversion.

## Coverage

The native ordinary-blast and wind-burst player paths now resolve Blast
Protection immediately before calculating their push. On each of 13 supported
correct-slot worn armor classes, current material/fixed traits and saved five-unit
progress produce a completed integer enchantment level. Source partial levels
are truncated, not rounded up. The live worn tag and complete stock definition
must match. Broken pieces still contribute, as the source enchantment hook has
no brokenness exclusion.

Add that level to the existing native Blast Protection level on the same piece,
then take the highest across all worn armor, including ordinary armor. The
source resistance is `clamp(maximum * 0.15, 0, 1)` using double arithmetic, not
the newer native sum across pieces. For example, two TiC level-two pieces supply
30%, not 60%. Level seven or higher cancels the native push before later mod hooks.

The bridge acts only with a positive source modifier contribution and a matching
native vanilla resistance baseline. Vanilla-only gear, unsupported native
enchantment formulas, a different observed native resistance, malformed data,
unsupported definitions and remote-client paths retain native behavior. The
highest-level source rule can be weaker than the host's summed vanilla rule
when ordinary enchanted armor is mixed with TiC modifiers; this is intentional
legacy-source aggregation, not an extra multiplier after native resistance.
Other attribute contributions that clamp to the same observed native value
cannot be distinguished by this baseline guard and are not covered.

Only the local resistance read changes. Blast exposure, damage, shields,
spectator/creative eligibility, native explosion-knockback hooks, packet push
and wind fall-distance handling are unchanged. Equipment, same-item edits and
resource reloads are read fresh at use time; no saved counter or derived
attribute is installed.

## Remaining work

Blast damage protection is not implemented. Held shields, mob armor, external
custom attributes, other direct impulse paths and full guest/LAN validation
remain outside this patch. Wind support follows the host's shared explosion
resistance behavior; wind charges were not part of the source game revision.

Sources: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`blast_protection.json`, `EnchantmentModule.Protection`, `EnchantmentModifierHook`,
`LevelingInt`, and legacy vanilla `ProtectionEnchantment` knockback damping.

Two regression cases were added but not run. Brief static review only. No tests,
builds, planner runs, CI inspection, version checks or merges. Actual runtime
hook matching, explosions, wind, save/reload and LAN remain unverified.
