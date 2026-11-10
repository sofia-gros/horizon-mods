# Worn Shell Gut effect immunity

This addresses the high-priority #379 Shell Gut conditional immunity item.
Source reference: TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`shell_gut.json`, `EffectImmunityModule`, `ModifierEvents.isPotionApplicable`,
`EquipmentChangeWatcher`, and Nautilus's shell-stat material trait.

## Source behavior

Each intact contributing armor piece supplies immunity points:

- Poison and nausea: the piece's Shell Gut level.
- Hunger: the piece's Shell Gut level plus one.

Points sum across eligible pieces. An incoming effect is denied only when its
zero-based amplifier is **less than** those points. Thus level 1 on one piece
blocks Poison I, Nausea I, and Hunger I–II, but not Poison II or Hunger III. Two
level-1 pieces provide four hunger points; one level-2 piece provides three.
Zero-level pieces provide no points, including no extra hunger point.

Existing active effects are not cured on equip or during ticking. An immune
application is rejected before replacing/extending an effect or running its
application callbacks. Stronger effects and unrelated effect types are unchanged.

## Connected paths

The two native player entry points are guarded: `give_player_effect` and
`add_player_effect`. Their existing callers cover mob hits, drinking, food,
tipped arrows, splash/lingering potion effects and native effect additions routed
through these functions. The Boolean add path returns false when immune. Item
consumption and other enclosing actions still continue normally.

The runtime reads the authoritative actor's current worn equipment, material and
fixed traits, integer upgrades, live worn-armor tag and complete source modifier
definition. Stock armor must be in its class's correct slot. Broken/unknown-wear
pieces are ineligible, unlike percent-style attribute modifiers. Unsupported
snapshots, changed definitions, malformed levels/counts and opaque wire patches
fail closed. Overflowing immunity sums do not grant a guessed immunity.

No saved/cached immunity counters are introduced. Removing or breaking gear,
repairing it, and resource reload therefore change the next application check
without a stale bonus. Only exact vanilla poison/hunger/nausea IDs are handled.

## Deferred and review

Held armor, non-stock armor classes, custom effect-immunity definitions, mobs
wearing equipment, direct mutations of native effect lists and source combinations
with other immunity modules are outside this patch. It does not add or prove
Nautilus-shell construction. Remote clients retain their native behavior; actual
save/reload, guest equipment swaps, packet synchronization and LAN are untested.

Brief static source/call-site review and formatting only. Three pure regression
tests were added for thresholds, stacking/overflow, namespace and source gates,
but were not run. No tests, builds, planner, CI, version checks or merges were run.
