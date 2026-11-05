# Skyfall worn attributes

Related to #379. Stacks on Dragonfall #755 to share its safe-fall projection.

## Implemented

- Canonical Skyfall on correctly slotted stock TiC worn armor contributes `-0.05 - 0.1 * level` to gravity with source float evaluation. Each piece gets a distinct transient native `MULTIPLY_TOTAL` modifier, so two level-one pieces multiply gravity by approximately `0.85 * 0.85`, rather than adding the reductions.
- The percent-style gravity module uses equipment-change hooks and remains active on broken armor. Its ordinary safe-fall attribute is disabled when that armor is broken.
- Intact armor contributes one block of safe-fall distance per level. Skyfall and Dragonfall share one native projection and one source range cap of 100 blocks, preventing separately capped bonuses from exceeding the source attribute range.
- Read current trait and upgrade levels through the material snapshot, check live worn tags and the exact source definition, reject ambiguous stacks, and remove owned modifiers when equipment or valid context disappears. No derived bonus is written to inventory or saved tool data.

## Limits and integration

Worn stock armor only. Held attributes, custom modifier definitions/attributes, non-stock armor, and end-to-end acquisition/save/reload/LAN validation remain separate. The native fall damage calculation retains its existing rounding/epsilon behavior described in #755; this does not claim exact source fall-event parity.

The existing Dragonfall equipment refresh owns the combined safe-fall contribution under its original transient modifier ID; do not add another Skyfall safe-fall modifier during integration. Skyfall's gravity IDs are distinct from Dreambound #764, so their per-slot total multipliers can coexist. Unknown Skyfall data clears its contribution without suppressing a valid Dragonfall contribution.

## Review

Brief static review and Rust formatting only. Three pure tests were added for per-slot gravity arithmetic, the shared safe-fall cap, and definition overrides; none were run. No builds, planner, tests, CI inspection, version checks, or merge were performed. Final combined validation should cover equip/unequip, broken armor, mixed Skyfall/Dragonfall/Dreambound, resource reload, and both host and LAN actors.

Source: TinkersConstruct commit `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, `data/tconstruct/tinkering/modifiers/skyfall.json`, `AttributeModule`, and `ModifierEvents` safe-fall handling.
