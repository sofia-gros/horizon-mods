# Fortune and Fortunate player harvest

Issue #379. This implements the tool-side Fortune loot effect, not the complete
Fortune, Fortunate, or Looting modifiers.

## Source and calculation

- The pinned [Fortune definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/fortune.json)
  supplies `minecraft:fortune` at the modifier's level on tools in the live
  `tconstruct:modifiable/harvest` item tag.
- The pinned [Fortunate definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/fortunate.json)
  supplies a flat one level whenever its modifier level is positive. Higher
  Fortunate levels do not add more harvest Fortune.
- [EnchantmentModule.Constant](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/modules/build/EnchantmentModule.java)
  adds to the existing enchantment level. Material traits and separately stored
  upgrades are combined once through the existing world-aware trait resolver.

## Bounded host connection

The `Game::block_loot_at` HEAD hook builds a temporary clone of the supplied
pre-wear harvesting stack, adds the effective Fortune enchantment, and runs the
native block-loot calculation once. Native tables determine probabilities and
Silk Touch precedence; this does not multiply already-generated drop counts.

The native named random sequence or level random, dynamic drops, block-entity
data, location, player-break flag, and native loot callback are retained. The
native callback also receives the projected loot tool. The original stack and
all its other components, including existing enchantments, stay unchanged.
Normal player breaks use this entry point. The Kama/Scythe crop adapter uses
the same native roll helper with tool-only enchantments; worn contributions
are covered separately in [worn Fortune](fortune-worn-harvest.md).
The existing area-mining path also delegates actual breaks
through the ordinary player path; no new area size or acquisition is added.

Only intact, count-one TiC tools with a resolved source material layout and the
live harvest tag are projected. Opaque wire patches, missing/failed snapshots,
malformed levels, duplicate enchantment keys, and changed active modifier
definitions fall back to the original native roll. This bounded implementation
also declines totals above 255 instead of truncating or overflowing. Definitions
are read at each roll, so there is no new reload cache or persistent state.

## Deferred and validation

- Worn Fortune harvest contributions are added by the stacked
  [worn Fortune change](fortune-worn-harvest.md). Luck attributes, fishing/sea
  luck and Fortunate's worn Looting are separate. Station acquisition and
  extraction are unchanged.
- This is not a general `ItemStack.getAllEnchantments` bridge. Other consumers,
  automation, mob harvesting, commands, explosions, broken tools, arbitrary
  modded tools, and custom modifier definitions are outside this scope.
- The cancellable HEAD mirrors the current native `block_loot_at` body only for
  a successfully projected tool. Later native changes or additional HEAD hooks
  at this method need a composition review; this is not a full host overwrite.
- Brief static review covered source fields, native roll/callback ordering,
  pre-wear stack capture, field-level borrows, type paths, and unchanged inventory
  ownership. Two pure regression tests were added but not run.
- No tests, builds, planner runs, CI inspection, version checks, or merges were
  performed. Runtime drops/distributions, resource reload, save restoration,
  native-hook composition, and LAN remain unverified.
