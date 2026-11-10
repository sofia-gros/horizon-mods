# Worn Crystalstrike candidate

This is a bounded implementation of the high-priority worn Crystalstrike entry
in #379. It does not implement held-shield Crystalstrike or claim full parity.

## Connected behavior

- Stock Travelers, Plate and slime armor is included only in its source class
  slot and while the live `modifiable/armor/worn` tag contains it. Material
  traits and saved upgrades are resolved from the current world.
- Each worn piece contributes a separate `multiply_total` attack-speed amount
  of `0.05f * level`, widened to double. The actual attack cooldown consumer
  applies it to both vanilla weapons and the existing TiC/Overwield replacement.
  The attack-speed attribute also receives four owned transient slot modifiers.
  Unequipping or changing the source definition removes those modifiers.
- `give_player_effect`, the shared mob/arrow effect entry, scales the duration
  of supported source nonbeneficial effects by the same per-piece multiplier.
  Neutral Glowing and Bad Omen are included; infinite duration is preserved.
  The result truncates to an integer and is at least one tick when modified.
- `knock_back_player` restricts incoming horizontal knockback to 32, 16, 8 or
  4 directions at total worn levels 1, 2, 3 or 4+. Strength and vertical motion
  are unchanged. Native cancellation, resistance and zero-direction fallback
  remain in the host. The implementation uses the source float rotation and
  sine-table index arithmetic.
- Broken pieces retain the two equipment-change attribute contributions,
  matching AttributeModule's lack of a brokenness condition. They do not
  contribute to knockback, matching ArmorLevelModule's `allow_broken: false`.
- Every use requires the exact pinned Crystalstrike module definition. Reloads
  and missing/malformed definitions fail closed. No new persistent item fields
  or caches are introduced.

## Remaining coverage

Held equipment, custom item classes/definitions and custom curative metadata
are outside this change. Effects delivered directly through potion drinking,
commands, clouds or other paths bypassing `give_player_effect` remain unchanged.
Only known source 1.20.1 milk-curable nonbeneficial effects are scaled; newer or
custom effects are not guessed. The custom bad-effect attribute itself is not
registered, so external attribute changes and other modifiers are not combined.
Knockback paths bypassing `knock_back_player` and cross-mod hook ordering remain
unverified. The host-provided sine table is used; full JVM numerical parity is
not claimed.

The pure-rule regression cases were added but **not run**, per the current
implementation-only instruction. Review was static only. Mixins application,
compilation, tests, CI, runtime, save/reload and LAN validation are deferred.
No CI status, version inspection or merge was performed.

Source reference: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`:
- [Crystalstrike definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/crystalstrike.json)
- [ModifierEvents](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tools/logic/ModifierEvents.java)
- [RestrictAngleModule](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tools/modules/ranged/RestrictAngleModule.java)
