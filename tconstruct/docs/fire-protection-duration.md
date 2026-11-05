# Fire Protection explicit-ignition duration

Related to #379. Stacked on the protection acquisition and Projectile Protection
attribute work. This adds the Fire Protection secondary duration for the native
player `ignite_for_ticks` path, without projecting vanilla protection points.

## Implemented

- Resolve live traits plus five-unit `tic_upgrades` progress on 13 correct-slot
  stock worn armor classes in the current worn tag. The source enchantment hook
  truncates the effective fractional level: four units alone do not shorten fire,
  five units provide level one. Unlike max-armor modules, this hook does not
  exclude a broken piece.
- Add the generated enchantment level to any existing native Fire Protection
  level on each piece, then take the highest across all worn armor, including
  ordinary armor. It is not the sum of equipped modifier levels.
- At explicit ignition, use source `ticks - floor(float(ticks) * level * 0.15f)`.
  The native maximum with existing fire time, creative immunity and thawing remain
  unchanged. No saved enchantments or native damage-protection values are added.
- Refresh transient per-player context after equipment updates, native player
  ticks and source reload. Missing/changed definitions or invalid records remove
  it. Remote clients do not add an independent projection.
- Guard the native burning-time baseline. Unsupported enchantment formulas or
  a changed live burning-time attribute retain the native path rather than
  discarding another mod or command's behavior.

## Boundaries

This covers lava ignition, lightning and native explicit player ignitions from
entity attacks that call `ignite_for_ticks`. Ordinary standing-in-fire contact
uses a separate `ignite_contact` calculation and is **not covered**. Mob equipment,
held shields, other direct fire-clock writes, guest state synchronization and
legacy entity-wide fire behavior remain outside this patch. Fire damage
protection is still unimplemented. Existing fires are not shortened retroactively.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`EnchantmentModule.Protection`, `EnchantmentModifierHook`, `LevelingInt` and
`fire_protection.json`; legacy vanilla `ProtectionEnchantment` duration formula.

Brief static review only. Two regression cases were added but not run. No tests,
builds, planner, CI inspection, version checks or merges were performed. Runtime
hook matching, save/reload and multiplayer behavior remain unverified.
