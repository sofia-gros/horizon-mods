# Nahuatl Lacerating primary melee

This #379 slice connects Nahuatl's statically registered Lacerating trait to
landed, fully charged primary attacks with the existing 15 intact main-hand
melee tools. It uses current material traits and the live melee tag. Broken,
remote-predicted, spin, malformed and invented purchased-Lacerating tools are
excluded. A dynamic definition overriding this static ID is not assumed to
have the original behavior.

The source implementation applies bleeding on every qualifying surviving
target, with no probability roll. Its nearby "50% chance" comment is stale.
Each trait level adds 40 ticks. A new effect adds 19 more ticks; an existing
effect with the same amplifier extends its remaining duration without adding
a second delay. Application happens before native weapon wear.

The registered harmful `tconstruct:bleeding` effect deals one damage whenever
its positive remaining duration is divisible by 20. Amplifier does not multiply
the damage or shorten the interval. It uses the native damage pipeline with
armor bypass and no knockback, preserves invulnerability time, and accumulates
the previous last-damage value as `ToolAttackUtil` does. Native resistance,
Protection, absorption, invulnerability flags and death protection remain in
that pipeline. Damage-indicator particles are emitted at half target height.
The timer itself remains owned by native effect ticking.

An ephemeral per-mob attacker marker credits these player-applied ticks to a
player. The existing native initial-hit guest map remains responsible for LAN
player identity. The marker resets when no bleeding effect remains and is not
saved, so save/reload and attribution after intervening hits require later
integration review.

## Explicit bounds

This implements the source baseline Looting amplifier of zero. Weapon/armor
Looting hooks, Looting-specific kill drops and hidden weaker-effect restoration
are not implemented. The host has no distinct Bleeding damage-kind enum, so
this uses Generic damage's armor-bypass/no-knockback behavior; custom damage-type
tags, bleeding-specific death messages and damage predicates are not claimed.

Shield/armor counters, projectile application, monster-held tools, PvP, unarmed
armor attacks, NoMilk removal immunity and infinite command-created bleeding
are outside this slice. The player attribution marker does not reproduce the
source's entire mutable `lastHurtMob` relationship. Material acquisition remains
separate. Do not treat this slice as completing all Nahuatl/Lacerating coverage.

Brief static review covered charged landed-hit gating, no random melee roll,
duration extension, fixed damage cadence, native hurt-frame restoration and
hook binding/constructor shapes. Pure duration/cadence regression code was
added but not run. No tests, builds, Mixins transforms, CI inspection, version
checks or merge were performed. Gameplay, composition, save/reload and LAN
remain unverified. Publication is paused.

Sources: Tinkers' Construct f4e77d43,
`LaceratingModifier.java`, `BleedingEffect.java`, `ToolAttackUtil.java`,
`TinkerModifiers.java`, `LootingModifierHook.java`,
`tinkering/materials/traits/nahuatl.json`, `damage_type/bleeding.json` and
`minecraft/tags/damage_type/bypasses_armor.json`.
