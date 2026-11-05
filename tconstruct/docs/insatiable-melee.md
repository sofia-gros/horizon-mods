# Insatiable: bounded primary melee connection

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`InsatiableModifier`, `TinkerEffect`, `TinkerModifiers`, `ToolAttackContext`, and
`ToolAttackUtil.performAttack`. Related to #379, not completion of that issue.

## Implemented

- Intact selected-mainhand TiC tools carrying live material/upgrade Insatiable
  and the current `modifiable/melee` tag participate in primary attacks on mobs.
  Incomplete resources, replacement JSON definitions, malformed levels, opaque
  component patches, and changed tools fail closed.
- Successful fully charged hits (`cooldown > 0.9`) add one effect level, up to
  five, and refresh the source 100-tick duration. A miss, rejected damage,
  uncharged hit, or extra attack does not grant a stack. The hit which earns a
  stack does not receive that new stack's damage bonus.
- The active effect adds `effectLevel * modifierLevel / 2 * toolMultiplier`
  conditional damage. It uses the existing shared conditional damage projection
  (legacy `tic_raging_melee_multiplier` field) and supported Jagged/Stonebound
  factors, then cooldown scaling. Source criticals add only a fraction of base
  damage, so the conditional bonus is not multiplied by the critical factor.
- Registers the beneficial source-colored `insatiable_melee` effect. Native
  player effects own duration, hidden effects, status updates and save/network
  serialization. Particles are disabled; the native HUD icon flag remains on.
- Milk keeps this `NoMilkEffect`, including hidden instances. The wrapper also
  preserves the existing Momentum effects and runs outside #559's milk HEAD,
  so combining those PRs does not make one remove the other's effect.
  Non-milk clears and explicit removals retain native behavior.
- Per-actor nested attack scopes are consumed once, invalidated on ambiguous
  recursion, and removed when the attack ends. Buff state lives on the player,
  not the tool, so switching tools does not erase it.

## Boundaries and review

Only primary melee attacks against mobs are connected here. The separate
[ordinary-arrow connection](insatiable-ranged.md) documents ranged coverage.
Sling launches, PvP, monster-held tools, unarmed attacks,
secondary/sweep damage, and the separate Kinetic armor effect remain unimplemented.
The native extra attack path does not receive this bonus. Full modifier ordering,
bit-identical mixed-trait floating-point evaluation, custom effective-level
modifiers, and source combat-event ordering are not claimed. Accumulation happens
at the native successful-damage return, before later host hit callbacks.

The existing projection must be present; this does not migrate old tools.
Source effect icon artwork is not provided. Native save/reload and LAN plumbing
is reused but not exercised, and native hidden-effect persistence limits remain.

Brief source/host static review and formatting only. Two pure regression tests
were added but not run. No tests, builds, planner runs, CI inspection, version
checks, or merges were performed for this change.
