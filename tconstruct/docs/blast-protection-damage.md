# Blast Protection damage reduction

Related to #379. Stacked on the Fire Protection post-armor damage bridge.
The shared transient protection stage now handles Fire and Blast together.

## Added coverage

- Intact stock worn armor and either-hand Travelers Shield, Plate Shield or
  Battlesign contribute 3 EPF per effective Blast Protection level on the live
  `tconstruct:protection/blast` damage tag. The stock tag includes
  `minecraft:is_explosion`. Fractional reinforcement progress is retained.
- Existing native enchantment points are counted once. The source virtual Blast
  Protection enchantment and its cancellation are represented by only the final
  TiC coefficient on intact tools, not an additional vanilla enchantment.
- Broken worn pieces retain 2 EPF per completed integer level on the live vanilla
  explosion tag, since source equipment iteration skips the modifier hooks but
  the virtual enchantment still exists. Broken held pieces add nothing.
- Fire and Blast contributions share one additional table and one native EPF
  stage. Overlapping custom tags add points before a single default 20-point
  clamp, rather than independently multiplying damage reductions.
- Damage tags and definitions fail independently per modifier: an unsupported
  Blast definition or unresolved Blast tag does not disable supported Fire.
  Unreadable damage-tag resources still invalidate the complete tag snapshot.
- Shared equipment/status/tick/reload refresh and parked-world invalidation
  remain. Replacing the table avoids accumulation across repeated updates.

## Boundaries

No additional knockback behavior is introduced here; that secondary effect has
its own bridge. [Magic damage protection](magic-protection-damage.md) now shares this stage.
Melee and Projectile damage protection, Overshield,
Boundless/extended caps, negative protection and full mixed-modifier ordering
remain outside this change. The inherited native bypass, wear/stat rounding,
source-only damage-ID projection, supported equipment and multiplayer limits
are documented in [Fire Protection damage](fire-protection-damage.md).

The common files and transient field/module names have been generalized from
Fire-specific names. Integrate this commit on its prerequisite, rather than
combining two independent cancellable hooks on the same native protection call.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`blast_protection.json`, `tags/damage_type/protection/blast.json`,
`ProtectionModule`, `EnchantmentModule.Protection` and `EquipmentContext`.

Two further regression cases cover overlapping tags and broken Blast armor
with an independently overridden source tag. They were not run. Brief static
review only; no tests, builds, planner runs, CI inspection, version checks or
merges. Runtime damage, hook matching, reload and LAN remain unverified.
