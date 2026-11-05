# Gold Platform recipe discovery

The existing four-platform crafting recipe now has its source recipe-book
unlock: possession of a current-world `forge:ingots/gold` item OR an unlocked
event for `tconstruct:gold_platform`. The hidden advancement preserves source
criteria names, OR requirements, parent and reward. Only the native recipe ID,
`recipes` event predicate, and native `items: "#forge:ingots/gold"` syntax change.

This reuses the bounded current-world tag bridge already shared by Gold Bars
and Duct discovery; no runtime tag or matching change is needed. Missing,
invalid or empty world membership fails closed. Tag-only changes take effect
on the next inventory event, without revoking progress.

Gold Platform remains an item used as a Basin cast and a Scorched Basin
crafting ingredient. Placement and connection behavior are still unavailable,
and the existing localized placement warning is retained. This unlock does
not register a block or change the recipe's four-platform yield, gold inputs,
casting behavior or Scorched Basin recipe.

Exact pinned TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` advancement
bytes and hash are in `tests/fixtures/gold_platform_unlock/`. Regression
coverage checks the source mapping, registered partial item, existing recipe
and packaged advancement. No tests or compilation were run; brief static
review only. Actual events, recipe-book UI, crafting, reload and LAN remain
unverified. No CI inspection, version check or merge was performed.
