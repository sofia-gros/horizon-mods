# Source melee area actions

This draft implements source circle attacks against mobs for Scythe and Kama,
source mob sweeps for Sword and Cleaver, and innate Knockback for Excavator
and Battlesign. The tested runtime is `6a2fc260c6109a6709da26bdd2e8c89ed30d6be2`,
transformed with official Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab`.

Linux gameplay confirmed circle primary and nearby secondary hits, distance
exclusion, identical low-charge scaling and per-hit wear; a nearly broken
Scythe stopped after its first secondary. Sword and Cleaver hit their primary
and nearby secondary once, with the source range difference; low charge and
sprinting suppressed Sword sweep. Hosted LAN confirmed a guest Scythe against
mobs. Battlesign and Excavator produced observed knockback velocities 0.7 and
1.2 respectively, with one and two durability consumed. A vanilla iron sword
regression passed. These are representative tests, not all weapons or targets.
A same-team mob was excluded from Scythe and Sword secondary attacks. A falling
critical Scythe hit retained a non-critical secondary hit; a critical Sword hit
did not sweep. Broader allied cases, LAN combinations and Windows remain
individually unverified. Expanded and Sweeping remain unfinished.

PvP primary and secondary targets are not connected. The current player sweep
queue stores the attacker position and hardcodes a three-block distance;
source Scythe needs distance from the primary target, and Cleaver needs a
four-block attacker distance. Guest sweeps are deferred until the acting player
is restored, so per-target circle attacks also need the original attacker's
item and durability to be handled in source order. The existing player-victim path has no `weapon_hit` / `weapon_wear` call. A
landed-hit wear hook now routes the source tools through the acting player, but
is only a prototype and does not establish PvP area or wear-order correctness. The tested official Mixins
`a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab` planner rejected a correctly
selected `MODIFY_CONSTANT` of the existing `sweep_players` distance with
`E_UNSUPPORTED_CONTROL_FLOW` due to a closure in that function. A separate
state-addition attempt on `Sweep` at `player_sweep` also failed planning because
that constructor contains a closure. This describes
one attempted injection, not all possible hooks. The draft does not replace
that limit with a misleading vanilla approximation.

Remaining acceptance includes player targets and PvP permissions, broader allied
and critical cases, additional LAN cases, and other modifier behavior. Off-hand attack, other modifier abilities, armor and ranged
weapons are future work. Crop shears loot remains unimplemented.

## Planner boundary evidence

Against the pinned Horizon workspace and Mixins CLI `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab`:

* `MODIFY_CONSTANT` targeting
  `horizon_client::game::lan_pvp::Game<'_>::sweep_players`, selector `9.0`,
  `ordinal = 0`, `require = 2` returned `E_UNSUPPORTED_CONTROL_FLOW`:
  `closures, async, try/?, unsafe and const blocks are unsupported injection contexts`.
* Adding state to `horizon_client::game::lan_pvp::Sweep` with constructor
  `Game<'_>::player_sweep`, selector `Sweep`, `require = 1` returned the same
  code and message; its constructor contains a `map_or` closure.
* `BEFORE_CALL self.weapon_hit` in `Game::attack_mob`, `ordinal = 0`,
  `require = 1` returned the same code and message. Independent `weapon_hit`
  HEAD, `attack_mob` HEAD/RETURN, `extra_knockback` RETURN and `pvp_land`
  RETURN probes were accepted.

In the present prototype a mob primary does **not** damage any player in its
area: the provisional mob selection only visits mobs. The player-primary path
likewise does not perform source area attacks. The separate landed-PvP wear
hook is provisional and does not fix either missing area path.

Further bounded probes: `Game<'_>::guest_attack` RETURN and
`Game<'_>::lan_guest_packet` RETURN in `lan_actions` both returned the same
closure `E_UNSUPPORTED_CONTROL_FLOW`. `Game<'_>::flush_sweeps` HEAD and
`Game<'_>::attack_guest` RETURN were accepted. The flush boundary occurs after
`self.lan` is restored and could host a separate player-action queue, but that
queue still needs a proven way to preserve the attacking guest's original tool,
per-target ordering and primary durability loss if the guest disconnects or
several hits are pending. Planner acceptance alone does not verify those
semantics.

A separate official planner probe accepted both HEAD and RETURN at
`lan_actions::Game<'_>::leave_guest`. HEAD runs while the guest is the acting
player; RETURN runs after the host state is restored. This may permit a safe
handoff at a narrower boundary, but applying damage to the host while the
guest is still installed would target the wrong player. Source wear ordering,
subsequent target eligibility and actor disconnect behavior have not been
implemented or validated through this candidate.

## Material attack speed and player base overrides

The active material attack-speed helper now adds the tool's saved speed minus
four to the player's current `minecraft:attack_speed` base value. A tool with
speed two therefore produces speed four when the player's base is six; the
ordinary base of four still produces speed two. The lookup and missing-base
fallback follow pinned Horizon `0c7b5b61`'s `main_hand_attack_speed`.
Existing Haste/Mining Fatigue factors are applied afterward, then the result is
clamped to 0–1024.

This matches the held attack-speed modifier in pinned TConstruct
[`AttributesModifierHook`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/hook/behavior/AttributesModifierHook.java):
`tool attack speed - 4` is an additive modifier, not a replacement for the
player's base attribute. The change is limited to that base composition. It
does not alter the existing item eligibility, held versus recorded attack-item
selection, or support arbitrary external attribute modifiers. Broken-item
handling is described below.

A lightweight Rust boundary harness executes the actual helper and checks
default/changed/zero/unavailable bases, potion ordering, clamps and native
fallbacks. A Python assertion confirms the hook remains enabled. These checks
do not compile the generated host or demonstrate in-game cooldowns, save/reload,
LAN synchronization or Windows behavior; those remain unverified.


## Broken material tool attack speed

For a held TConstruct stack with a saved material attack-speed stat, the helper
now suppresses the tool's speed modifier when the existing retained-durability
predicate marks it broken: a nonzero `tic_broken` byte or damage at/above a
positive `tic_max_damage`. It returns the player's current base with the existing
potion factors and clamp. It does not fall through to the host's static item
speed, which could reapply a tool modifier. Repairing the damage and clearing the
broken flag restores the saved material speed on the next query.

Pinned TConstruct's `AttributesModifierHook.getHeldAttributeModifiers` wraps its
held modifiers in `!tool.isBroken()`. This slice fixes only attack speed for the
existing material-stat path; general broken melee damage and wider melee eligibility remain separate.
The narrow Raging flag check is described below. It does not rewrite stack damage, flags or
the player's attributes.

The expanded boundary harness includes the actual `source-ranged-wear.rs`
predicate alongside the actual injected helper. Eleven Rust cases pass, covering
break/repair, both broken indicators, custom and fallback bases, potion factors,
clamps and existing native fallbacks. Generated-host compilation, gameplay,
save/reload, LAN and Windows remain unverified.


## Raging respects the retained broken flag

The active melee damage-return helper now rejects Raging's conditional bonus
when the shared retained-durability predicate sees a nonzero `tic_broken` byte,
even when current damage remains below the maximum. Its existing positive
`tic_max_damage` and damage-limit checks are preserved. Clearing the flag on an
otherwise intact item restores the bonus; no stack state is changed by this
read-only calculation.

Pinned TConstruct
[`ToolAttackUtil.canPerformAttack`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/helper/ToolAttackUtil.java)
excludes broken tools before their conditional damage hooks, and
[`ToolStack.isBroken`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/nbt/ToolStack.java)
reads the persisted broken flag. This slice suppresses the Raging bonus only;
it does not implement source-wide broken attack cancellation or alter the
existing base damage, percent-trait calculation, critical handling or cooldown.

A lightweight harness executes the actual damage-return helper and shared
broken predicate. Six Rust cases pass, covering flag-only breakage, negative
nonzero flags, restoration, maximum damage, invalid maximums, actor health,
existing living/spin/trait gates and unchanged secondary critical handling.
Actor and trait inputs and the unrelated percent-trait helper are boundary
doubles. The guest-shaped case is not a LAN test. The enabled RETURN hook is
also checked; generated-host compilation, gameplay, saving, LAN and Windows
remain unverified.

## Broken material melee-damage projection

The existing matched-item melee projection now sets both saved material damage
and its Jagged/Stonebound wear bonus to zero when the shared retained-durability
predicate marks the held tool broken. It still subtracts the host registry's
current native item damage inside a cloned attribute map, rather than falling
through to the native tool damage. Player base damage, additive effects and
multipliers remain in the same attribute evaluation. Repairing the damage and
clearing the broken flag restores the saved material contribution on the next
query. Neither the real attribute map nor the stack is modified.

This follows the `!tool.isBroken()` guard around the held attack-damage modifier
in pinned
[`AttributesModifierHook`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/hook/behavior/AttributesModifierHook.java).
The slice retains the existing requirement that held item ID equals the host's
recorded attack-item ID and that a saved floating-point `tic_attack_damage`
exists. Mismatches and unavailable projection state retain their existing native
fallback. It does not implement source-wide attack cancellation or expand
item eligibility, attribute override support, or conditional damage ordering.
Pinned Horizon `0c7b5b61` updates equipment before recording `attack_item` in
`game/hand.rs::tick_hand`; this change uses the existing projection boundary
rather than adding an equipment refresh hook.
The projection retains its existing assumption that the native item contribution
in the player map matches `attack_item_info().attack_damage`. Custom stack-level
or external replacements for that native modifier are not validated here.

The lightweight harness executes the real projection helper and broken-state
predicate. Seven Rust cases pass, covering flag-only and damage-limit breakage,
repair, base/additive/multiplier preservation, changed native item damage,
unchanged mismatch/error fallbacks and suppression of wear traits. Attribute
map and trait behavior are boundary doubles; this is not a generated-host or
LAN test. Generated-host compilation, gameplay, save/reload, LAN and Windows
remain unverified.
