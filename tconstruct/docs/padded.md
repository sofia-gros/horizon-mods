# Dagger Padded candidate

This bounded #379 implementation follows the source Padded modifier: it reduces
knockback **dealt to the target**, not knockback received by the dagger holder.
The tracking issue's initial incoming-knockback description is inaccurate.

## Connected path

For an intact stock Dagger in the selected main hand, the authoritative primary
`attack_mob` path resolves material/fixed traits plus saved Padded upgrades.
It requires the live melee tag and exact pinned Padded module definition.
It then divides the complete source base knockback by `2^level`: attacker
attack-knockback attribute / 2, plus the living-target 0.4, plus the charged
sprint 0.5 when present.

A level-one ordinary hit therefore applies 0.2 rather than vanilla's 0.4.
A charged sprint hit produces 0.45 total. At higher levels or lower totals,
where the total falls below 0.4, the native default hurt push is temporarily
suppressed, the victim's original knockback resistance is restored, and the
reduced push is applied in the attacker's facing direction after a landed hit.
For totals at least 0.4, the native default push is retained and only the
remaining extra push changes. Failed hits restore resistance without adding a
push. Positive post-hit remainders slow the attacker horizontally by 0.6 and
end sprinting, following ToolAttackUtil.

The operation wrappers call the original damage/extra-knockback operations once
and preserve other wrappers. They target `attack_mob` rather than rewriting the
macro-containing internals of `hurt_by_player` or `extra_knockback`. Transient
stacked contexts match the acting player and target mob; unsupported nested
attacks mask the outer context. No item data or saved mob attributes are added.

## Remaining coverage

This is not a general melee hook dispatcher. Offhand attacks, PvP, other tools,
projectiles, secondary attacks and the `knockback_slings` force module are not
connected. Native Knockback enchantments, other active source knockback modules,
unsupported data and Sulfur Cube's newer special launch path leave the original
behavior in place. Cross-mod attribute changes or nested damage while the
victim's resistance is temporarily suppressed remain unverified. Panic recovery,
full source modifier ordering and external runtime overrides are not provided.

Two pure arithmetic regression cases are included but **not run**. Review was
static only, with formatting and whitespace inspection. No compilation, tests,
CI inspection, version checks or merge were performed. Mixins application,
runtime, save/reload and LAN validation remain for the final pass.

Pinned sources:
- [Padded definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/padded.json)
- [KnockbackModule](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/modules/combat/KnockbackModule.java)
- [ToolAttackUtil](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/helper/ToolAttackUtil.java)
- [ToolAttackContext](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/context/ToolAttackContext.java)
