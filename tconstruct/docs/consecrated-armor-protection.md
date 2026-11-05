# Consecrated armor protection (#379)

Adds Silver's canonical Consecrated protection module on supported worn armor
and held shields. Each intact modifier level contributes 1.25 EPF when the
verified causing mob is undead. Source `ProtectionModule` tests the causing
entity (`DamageSource.getEntity`), not the direct projectile, so an undead
archer's arrow can qualify.

## Coverage

Current material traits and independent saved levels combine once. Exact source
modifier JSON, stock equipment/slot mapping, current worn/held tags, item count,
material data and durability are checked. Broken or exhausted pieces add zero;
malformed or unsupported data removes this slice's points. No item enchantment,
new crafting recipe or saved derived statistic is created.

The preceding Scorch hit scope now records two independent facts from the
identified mob: native `fire_immune` and `undead`. Both predicates can contribute
on the same hit and join native protection once, before the final cap. Neither
predicate substitutes for the other. Prior context is restored after the call,
including canceled hits, without adding a second wrapper to the same native call.

Supported paths are `mob_hits_player` with a recorded projectile owner and
native projectile source; explicit `mob_hits_player_caused`; `CubeHurtPlayer`;
and `wither_skull_hit`. A known non-undead mob contributes no Consecrated bonus.
Explicit absence, unloaded owners and ordinary melee with only a source position
remain unsupported. The nearest-mob guess is never used. Generic `/damage` and
other unwrapped paths are not claimed. Consecrated Skull's nested effect is
separate and is not implemented here.

Equipment/status refresh, server-player tick and source reload recompute the
owned points. Actual damage is server-authoritative and retains native
armor/Resistance/bypass handling and the prior Boundless cap. No native edits
or provenance-blocked fluid draft are included.

Base: `feat/scorch-protection-causing-attacker-379` (`942c9acd`), retaining its
native cap dependency `5e47547e2`. Sources: TConstruct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2` (MIT), `consecrated.json`,
`ProtectionModule.getProtectionModifier` and equipment iteration. Original
adapter code; no upstream implementation copied.

Brief static review and whitespace checks only. Pure definition/arithmetic
regression cases were added but not run. No tests, builds, planner transforms,
CI checks, version checks, merges, publication, live combat or LAN validation.
The identified-attacker limitation remains an open parity gap.
