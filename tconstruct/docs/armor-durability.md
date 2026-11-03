# Material armor damage and repair

> **Provisional build:** Some effects and operations described below are unavailable or under verification. Earlier test results refer to their stated revisions; see [current limitations](provisional.md).

This draft targets material armor damage mitigation, per-slot durability,
broken-item retention and disabled broken attributes, and restoration after
Station repair. Existing Horizon damage-source, armor, enchantment and LAN
paths remain authoritative. Registered items and displayed attributes alone do
not demonstrate these actions.

The representative acceptance plan covers multiple material armor slots,
armor/toughness and Ductile, fire and armor-bypass damage, last durability and
broken retention, repair/re-equip/save, hosted LAN guest, and vanilla armor
wear/break regression. Other source armor modifiers and equipment effects,
projectile firework stats, PvP area attacks, shears loot and trim remain open.

The implementation captures the existing `durability_loss` result during
`hurt_armor` after the shared Dense adjustment, then applies it to matching
unchanged source armor slots that have per-item material durability and no
fixed registry maximum. It uses the existing retained-damage helper; an
item that breaks has equipment attributes recomputed. Source repair lookup
now includes repairable armor plating, material-stat definitions, and fixed
material / statless-part repair modules. Registered source armor currently has
no registry maximum; fixed-max source armor is intentionally left on the
native path rather than being damaged twice. Protection modifiers and other
armor-specific abilities are not implemented by this change.

Official Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab` transformed runtime `7fb00d0fb7f9ccda0d13df3971d03fcc1aa05d14` in 197.39 seconds; the Linux client/ctl build succeeded in 20.32 seconds. Two Rust slot-mapping tests and the 44 TConstruct plus 7 root Python tests passed.

Representative Linux game tests confirmed the first hit uses the equipped armor and toughness before a last-durability helmet breaks; the next hit uses only the remaining chestplate. Both items consumed one point, the helmet stayed equipped, and an iron Station repair restored its armor and toughness. Save/reload, a steel-plating replacement with Ductile toughness, Traveler's Helmet statless leather repair, Slime Wings fixed phantom repair, a fire-resistant slot, magic armor bypass, hosted LAN guest break/sync, and vanilla iron armor break were also checked. Some items used synthetic material data to isolate the behavior. These are representative cases, not all armor families, modifiers, damage types or platforms; Windows remains unverified.

## Heavy armor stat

When a material-bearing armor item is assembled or rebuilt at a Station, the
Heavy trait adds `0.15 * level` to its knockback-resistance stat, scaled by the
tool definition's additive multiplier. Netherite's separate `0.05` stat bonus
combines with Heavy before the stat is clamped to the source range of 0–1.
Replacing the material recalculates the value from the base stats. The existing
armor attribute path reads the resulting value when worn.

This implements only the armor-stat portion of the original [Heavy
definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/heavy.json).
Its movement-speed and gravity attributes, the non-armor resistance attribute,
and the Creative resource-generation path require separate work. Lead is a
compatibility material, so the change does not establish its natural
acquisition. Rust syntax was checked; unit, plan, build and gameplay are pending.
Verify zero/one/multiple levels, non-unit multipliers, Netherite combinations,
clamp, equip/unequip, material swap, save/reload and LAN, using labelled
fixtures when needed. Armor wear remains subject to its separate hold.

## Wool knockback-resistance trait

The source Wool armor trait adds `0.1 * level` to the knockback-resistance
stat. Station rebuilds now combine that additive amount with Heavy and
Netherite using the tool's stat multiplier before clamping to 0–1. It uses the
existing worn-armor attribute path; non-armor effects are not added.

This depends on the material-stat trait resolver for equipment whose definition
uses `material_stats`. Rust syntax was checked; unit, current-target plan/build
and gameplay remain pending. Verify Wool material selection, zero/one/multiple
levels, source fallback, Heavy/Netherite combinations, non-unit multiplier,
clamp, equip/unequip, part replacement, save/reload and LAN. Creative resource
generation remains separate. Source:
[ModifierProvider](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/tools/data/ModifierProvider.java).

## Slime material and Overslime capacity

For the pinned source slime armor definitions, `SlimeStats.apply` adds the
scaled slime material durability to both durability and the Overslime stat.
The capacity calculation now includes that material contribution before the
Overcast and global durability multipliers. The source also has an
`overslime_capacity` field for its tooltip; its pinned `apply` method uses the
durability field for the stat contribution, so this calculation follows that
method rather than the tooltip value.

Rust syntax was checked. Unit tests, combined build and gameplay are pending.
Verify each slime armor scale, material replacement, Overcast/Overforced,
global durability multiplier, capacity clamping, existing shield contents and
save/reload. Armor wear and Overslime writeback remain subject to the separately
documented provisional hook limitations.
