# Battlesign Bonking: primary mob runtime

High-priority issue #379 follow-up, stacked on shield runtime #851 and
Blockade acquisition #865. This is a bounded main-hand Battlesign path, not
complete Bonking support for every tool, hand or target.

## Implemented

- Reuse the normal blocking item-use lifecycle. The Battlesign continues to
  block while charging. Native Stop cancellation and hand changes are retained.
  Capture draw time at start: 30 divided by the source tool attack-speed stat
  for a melee weapon (25 ticks at the standard 1.2 speed).
- Release uses the source partial-charge curve. The same actor, main-hand slot,
  item, materials and saved upgrades must still match. Ordinary durability or
  Overslime changes while blocking do not invalidate the charge; broken tools
  and changed upgrades do.
- Select the nearest living mob intersecting the source five-block ray and
  broad-phase box. Use the inflated target box and compare target-eye distance
  with the intervening block center, as SlingKnockbackModule does. Native
  block outline clipping is reused. Unloaded ray columns fail closed.
- Call the existing primary mob melee body once. A temporary, nested-call-aware
  context supplies the exact charge, the Bonking 1.5x base-damage contribution
  and suppression of ordinary melee push. The source additive critical bonus
  is preserved rather than multiplied again. Normal attack callbacks, damage,
  successful-hit wear, exhaustion and network work stay on that native path.
- Restore the temporary context before the separate sling push. The canonical
  Battlesign Knockback trait contributes to force: at charge one and level one,
  force is 3.75. Native mob knockback handles resistance, velocity and network
  synchronization. The target is resolved again by stable ID after callbacks.
- Add the source one extra wear with Bonking as its cause, 0.2 exhaustion,
  three-tick item cooldown and success/failure Bonk sound pitches. Wear uses
  the existing Overslime chain, retains broken tools, and refreshes equipment
  if that extra point breaks the tool. Creative skips wear.

The live tool definition, Bonking/Blocking/Knockback definitions and target
sling tag must have their supported source forms. Unknown modules, competing
interactions, extra combat hooks, native enchantments, consumable/cooldown
components and conditional stats are rejected. Known simple build/repair/wear
modules and canonical Blockade are allowed. Knockback is bounded to levels
0..16 for this first slice. The allowlist does not cover all material choices.

## Remaining

Offhand Bonking, PvP, armor/helmet use, non-Battlesign application and advanced
SlingForce/SlingAngle/SlingLaunch combinations remain separate. The custom
Bonk attack particle and separate tool-break animation are not implemented;
native melee particles remain. Source fast-weapon invulnerability-time
adjustment and every native/source mob-specific attack difference are not
claimed by this slice. Runtime, visuals, save/reload, integrated/dedicated host
and LAN behavior require the later combined pass.

The Bonk sound asset is copied unchanged from SlimeKnights/TinkersConstruct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`src/main/resources/assets/tconstruct/sounds/bonk.ogg`, under the existing
[TConstruct license](../third_party/tconstruct/LICENSE). Rules follow that
commit's `SlingKnockbackModule`, `SlingModule`, `ToolAttackUtil` and generated
modifier/tool-definition resources. No paused unpublished branch was used.

Two pure tests were added but not run. Validation is brief static review,
Rust formatting, TOML/sound JSON parsing and whitespace inspection only.
No tests, build, planner, CI, version checks or merge were performed.
