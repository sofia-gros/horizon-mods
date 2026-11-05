# Warping ability acquisition

Acquisition slice of issue #379's high-priority held Warping task. Adds the
pinned source recipe to the existing authoritative Station upgrade transaction.
Charging, teleporting and its durability cost are a separate slice documented
in [Held Warping runtime](warping-held-runtime.md).

- Ender Slime Vine, Ender Slime Crystal, Ender Slime Vine, and two Ender
  Congealed Slime inputs grant one Warping level for one ability slot. The
  ordinary recipe needs five separate input slots, so it is an Anvil operation,
  not a three-input Station operation.
- The original durability AND interactable/charge tool tags are evaluated from
  the current world snapshot. The recipe checks both saved and material trait
  levels. An existing positive Warping trait or saved level blocks application.
- An extracted Warping modifier crystal can be reapplied through the normal
  single-input Station path, but still requires the same tags, cap and free
  ability slot. Zero-count crystals and extraction inputs are rejected.
- Extraction removes the saved level, returns its modifier crystal and dry
  sponge, and frees the occupied ability slot through existing slot accounting.
  It uses the existing generic modifiable/non-unsalvageable extraction rules;
  material traits are not removable upgrades.
- Saved integer levels are bounded to 0..1. Malformed upgrade containers or
  Warping values are rejected rather than overwritten. Part-replacement slot
  validation now counts the Warping ability. Normal names/tooltips include it.
- The recipe and dynamic modifier JSON must match the supported source forms;
  authoritative assembly also requires resolvable current material traits.

Source: SlimeKnights/TinkersConstruct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, generated
`recipes/tools/modifiers/ability/warping.json` and
`tinkering/modifiers/warping.json` in the bundled reference archives.

The new selection ID is appended, preserving all earlier indexes in this
branch. Final integration must reconcile other parallel appended modifiers,
including Worldbound and the locally held defense acquisition changes.
No paused branch was incorporated.

Four pure tests were added but not run. Independent static review confirmed
the pinned JSON and transaction paths, and corrected a zero-level UI row.
Only a brief static review and diff
whitespace inspection were performed. No tests, builds, planner, CI, version
checks or merge. World acquisition of the required Ender ingredients, in-game
Anvil/Station interactions, persistence and LAN remain unverified; the existing
transaction and safe tool-mutation paths are reused rather than replaced.
