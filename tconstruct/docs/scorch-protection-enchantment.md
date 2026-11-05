# Scorch Protection's Fire Protection enchantment

Related to high-priority #379. This patch covers the enchantment module of
Scorched Stone's Scorch Protection, not its full attacker-conditioned protection.
It extends the existing fire-duration/contact and shared damage implementations.

## Added coverage

- Resolve the exact stock Scorch Protection definition, live material traits and
  saved integer modifier levels. Definition changes disable only this module's
  contribution; the ordinary Fire Protection definition remains independently
  guarded. No vanilla enchantment or derived NBT is written to the item.
- The source generates Fire Protection at one enchantment level per Scorch
  Protection level. On each supported worn armor piece, add it to the ordinary
  Fire Protection modifier's truncated effective level and any real native
  Fire Protection enchantment. Then use the highest per-piece total, not a sum
  of all equipment, for explicit ignition and fire-contact duration reduction.
- Preserve the existing source duration arithmetic, native baseline guards,
  fire-clock behavior, effect/equipment/tick/reload refresh and remote skip.
  Broken worn armor still generates this enchantment in the source, so it also
  participates in fire duration. Held shields do not contribute to vanilla
  armor enchantment processing.
- Preserve the source's broken-armor edge case in the damage stage. Intact
  Scorch Protection cancels its virtual vanilla Fire Protection EPF, so no
  unconditional fire-damage bonus is added. Broken worn armor skips that
  cancellation but still generates the enchantment: add 2 EPF per level only
  for the live `minecraft:is_fire` tag. Broken held shields add nothing.
- The broken-worn contribution combines once with native and other modifier
  protection at the shared default-cap stage, not a second damage multiplier.

## Boundaries

The later [identified-attacker slice](scorch-protection-attacker.md) adds bounded
coverage for explicit causing IDs and verified projectile owners. The following
paragraph describes the original enchantment-only patch.

The main Scorch Protection module adds 1.25 EPF per level when the causing
attacker is fire-immune. It remains unimplemented here because reliable causing
entity identity is not available for every native hit. Fire damage alone is
not equivalent to that condition, and this patch does not substitute it.

Other custom worn/held tools and mod-defined enchantment formulas are outside
scope. Existing fire-contact clock and protection rounding/stat limits remain.
The branch inherits Depth Protection's native signed-protection dependency
(`09f0179db`); this patch itself adds no native hook or state.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`scorch_protection.json`, `EnchantmentModule.Constant/Protection`,
`ModifiableArmorItem.getAllEnchantments` and `EquipmentContext.iterateTools`.

Three regression cases were added for per-piece enchantment addition, exact
module guards and the broken-worn exception. They were not run. Brief static
review and `git diff --check` only; no tests, builds, planner runs, CI inspection,
version checks or merge. Publication remains paused.
