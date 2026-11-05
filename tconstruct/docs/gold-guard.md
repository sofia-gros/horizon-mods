# Gold Guard worn effect

Rose Gold Skull's Gold Guard now supplies the pinned source's max-health bonus
and level-2 piglin-neutral flag. This does not complete skull acquisition or the
broader armor modifier backlog.

Source: TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`GoldenAttributeModule`, `ModifiableArmorItem.makesPiglinsNeutral`, and bundled
`gold_guard` / `golden` modifier definitions.

- Gold Guard level 1 activates one +4-health base bonus. Every piglin-neutral
  worn armor slot adds another 4. The bonus is once per wearer, not multiplied
  by Gold Guard level or repeated copies. Activation requires the armor item’s
  proper class slot; neutral counting follows the source worn-slot check.
- Gold Guard level 2 makes its tool piglin-neutral and counts its own slot.
  Source Golden also makes a tool neutral. Ordinary non-TConstruct items use
  the host's native `minecraft:piglin_safe_armor` tag. Held items do not count.
- This source equipment-change effect and its volatile neutrality flag do not
  depend on tool brokenness. Generic armor stat brokenness is unchanged.
- Health is clamped, never healed, after an actual bonus change or removal.
  Unequipping, clearing equipment and returning to an ordinary helmet remove
  the effect. The modifier is transient, not an extra saved custom tracker.
- Current material traits, upgrades, worn tag and exact source definitions are
  required. Unknown/malformed source data fails closed. Recipe snapshot
  publication explicitly refreshes the effect after reload; unavailable or
  changed definitions remove an existing bonus. Equipment setup reconstructs
  the effect when a player/world is loaded.

The host applies this at the end of equipment attribute collection, after
ordinary equipment and enchantment effects. It owns one separate transient
modifier, avoiding a temporary removal/re-add health clamp on every equipment
refresh. Remote clients do not mutate player attributes through this hook.

Validation includes pinned JSON equality, full pinned-original target planning,
real host ItemStack/NBT adapter tests of the unchanged production helpers, and
CI's full staged-client compile plus named pure tests and adapter invocation.
The adapter mocks Game attributes and trait resolution; it does not prove live
attribute transport, actual save/network round trips, reload GUI or LAN. These
remain unverified until host/gameplay tests complete. Unsupported non-stock
TConstruct armor and custom modifier definitions are not approximated.

The adapter runner verifies the exact clean pinned Horizon checkout, resolves
workspace membership offline, rejects fetched dependencies outside the pinned
host lock (including checksum drift), and executes tests with `--locked --offline`.
