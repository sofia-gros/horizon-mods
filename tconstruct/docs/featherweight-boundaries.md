# Featherweight armor boundaries

This note records the unresolved protection and use-movement operations on the
formal target Horizon `0c7b5b61a5e746d621afffba919b12aeccf9c0fa`, against
TinkersConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. The existing
armor movement-speed attribute candidate is separate; this note does not add
these two effects or claim a game pass.

The source `ModifierProvider` adds protection -0.625 and item-use movement
speed +0.05 per level on modifiable armor. In source `ToolEvents.livingHurt`,
the modifier runs after vanilla enchantment protection is collected and before
`ArmorUtil.getDamageForEvent`. `ArmorUtil.getDamageAfterMagicAbsorb` supports
negative protection: with no armor, enchantments or other modifiers, damage 10
and level 1 becomes 10.25, and two levels become 10.5. With vanilla protection
4, level 1 leaves 3.375 protection before the source cap. Damage-type bypass,
armor wear compensation, other protection modifiers and their order must be
retained. The host `Game::apply_effect_status` builds a cached per-damage-type
vanilla enchantment table, while `horizon_world::damage_reduction` applies that
table only when protection is positive. Appending a negative number to the
existing table cannot produce the source increase. A correct damage boundary
must accept the signed combined modifier for each applicable source and keep
the source armor-wear and hook order; damage-type or enchantment bypass must
not be guessed.

For use movement, source `ToolClientEvents.handleInput` applies the player's
use-speed attribute plus the held tool and armor-stat contributions, clamped
to [0, 1], while the player is using an item and is not a passenger. One
Featherweight level with the source default 0.2 and no other contributions is
0.25; two are 0.30. Existing host `Game::sync_player_body` assigns the active
item's use speed into player motion; `guest_replay` separately assigns guest
speed. `MotionWorld` then applies that value during input modification. The
sync method's unrelated Snow Boots site was rejected by the official plan;
this is not proof that every other boundary is unavailable. A safe connection
must preserve the actor's armor and active hand, existing source tool/armor
contributions, passenger condition, local prediction and guest replay, and
must reset naturally when use ends or equipment changes. Altering raw input
before the host's own slowdown can change normalization or networking.

Verify one and multiple armor pieces, broken/equipped/removed, other protection
and use-speed modifiers, source damage-type bypass, absorption/armor and wear,
use/stop/release, both hands, vehicle, local/guest actor separation and replay.
Plan, type and gameplay remain unverified. A common signed-protection boundary
and actor-specific post-sync movement speed are candidate integration needs,
not claims that the formal host cannot be extended.

A subsequent local-only candidate reads the already applied, source-tagged
`featherweight_movement/{slot}` amounts from the player's movement attribute
and adds them once to the active item speed at
`MotionWorld::controlled_simulation_tick_with_sprint_window`. The formal host
`tick_remote_level` first calls `sync_player_body`, which refreshes the base
item speed for that local tick; passenger, server-copy and replay paths are
excluded. This preserves the item speed supplied by the host, but does not
assert that all original held-tool or armor-stat speed contributions are
implemented. Guest replay and the server-authoritative guest movement remain
open. The HEAD injection and source attribute sync have not passed official
plan, type or gameplay tests. Verify repeated ticks do not compound, 0.2 →
0.25 and 0.30, clamp at 1, broken/remove/equip, use/stop, vehicles, local
world and a guest client, original held-tool speed modifiers and unrelated
movement attributes; verify guest replay separately before claiming LAN parity.
