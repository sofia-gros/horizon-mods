# Looting from lethal ordinary arrows

Issue #379, stacked on PR #799. Adds a bounded launch-time Looting snapshot for
ordinary TiC Longbow/Crossbow arrows. The native loot roll still determines the
drop amounts, probabilities, equipment drops and ordering.

## Source contract

[ModifierLootingHandler](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/helper/ModifierLootingHandler.java)
handles projectiles differently from melee. It reads their captured modifier
capability, evaluates a `DummyToolStack` whose item is air, and starts at zero
instead of preserving the current mainhand's native Looting. It then adds the
holder's current armor contributions. The source Looting JSON explicitly allows
air as well as melee tools for this purpose.

This extension captures only positive `tconstruct:looting` levels, at most 255,
from intact count-one source Longbow/Crossbow stacks with live ranged tags and
the exact source Looting definition. Material traits and stored upgrades are
combined once. The separate `luck` modifier also has a weapon-looting module;
launchers containing it are conservatively excluded rather than silently losing
its contribution. Zero-Looting source projectiles are not intercepted.

## Native bridge and identity

The existing source launch context writes the level only into the native
retained firing-stack copy, as `tic_looting_arrow_level`. Actual launcher and
ammunition inventory are unchanged. Glowing now fills a missing retained weapon
instead of replacing the native pre-wear snapshot, matching the same fix in the
Spectral and Insatiable ranged PRs.

The native damage record stores arrow damage without the projectile object.
After a successful lethal arrow hit, the bridge records the victim UUID,
shooter UUID, native damage age, damage origin and captured level. It requires
the projectile owner to be the currently routed player, with a matching owner
UUID when present, and never substitutes the host for a departed guest.

The later death roll consumes that record once and verifies all identities and
damage fields again. A numeric entity id alone is insufficient. Records for
missing or changed deaths are pruned when another captured lethal hit arrives;
clearing the world chunk map clears the pending map. There is no persistent
death-bonus cache.

The existing PR #799 actor resolver reads current worn Looting/Looter/Fortunate
bonuses. The native killer snapshot receives captured projectile Looting plus
those worn bonuses, starting from zero. Current melee modifiers and current
native mainhand Looting are not added. A real nonempty current mainhand is still
required to carry this temporary enchantment view; no fake inventory item is
invented for an empty hand. Source definition changes fail closed.

## Limits and review

- Only successful lethal ordinary-arrow damage is captured. Nonlethal hits,
  delayed effect kills, already processed deaths, PvP, mob shooters, rockets,
  throwing weapons, slings, zero-Looting projectiles and Luck-bearing launchers
  remain outside this extension.
- Empty-mainhand and cross-actor deaths during another guest's turn remain
  outside the parent bridge. Custom modifier hooks or arbitrary modified native
  enchantment definitions are not emulated.
- The pending hit-to-death record is not saved. Saving/unloading between the
  killing hit and its loot roll, retained-arrow restoration, resource reload,
  distributions and LAN need final integrated validation.
- Final integration must combine the launch calls from this PR, Spectral and
  Insatiable rather than replacing one with another. Retain a single shared
  Glowing snapshot-preservation change and a single parent death hook.
- Brief static review covered the native hurt/death timing, actor/victim UUID
  checks, exact damage matching, source projectile reset rule, non-mutating
  inventory ownership and world-clear cleanup. One pure regression test was
  added but not run. No tests, builds, planner, CI inspection, version checks or
  merges were performed.
