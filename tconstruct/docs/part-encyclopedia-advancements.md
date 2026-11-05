# Part and Encyclopedia acquisition milestones

Adds two source inventory advancements with their existing parents:

- `tools/make_part` below Part Builder tracks the current-world
  `tconstruct:parts` tag. This includes the source's nested
  `tconstruct:parts/bartered` membership; it is not a fixed Pick Head check.
- `foundry/encyclopedia` below Fantastic Foundry tracks possession of the
  registered Encyclopedia. Its existing source Table casting recipe consumes
  one Book and 90 mB Molten Gold, with 57 ticks cooling.

Both retain source criterion names, requirements, translations, parents,
visible display and toast. Make Part is a task with chat disabled;
Encyclopedia is a goal with chat enabled. Neither grants rewards. The book's
recipes, item registration, reader and partial-content notices are unchanged.
This does not make the Encyclopedia's remaining chapters complete.

The part predicate uses native `items: "#tconstruct:parts"`, and both icons
use native `id`. The source Make Part icon also specifies wood-material NBT.
That display-only preview is omitted because host advancement icons accept
an item ID only, consistent with other materialized-tool advancement icons.
The acquisition predicate itself does not require a wood part.

The bounded advancement bridge now manages `tconstruct:parts`. Candidate
indexing and final matching use the same resolved current-world membership,
including datapack replacement. Missing/invalid/empty snapshots fail closed
without stale native fallback. Reload invalidates the inventory cache but
does not synthesize an inventory event or revoke already earned progress.

Exact source fixtures, paths, SHA-256 and attribution are under
`tests/fixtures/part_encyclopedia/`, from Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`.
Source translation, parents, existing casting route, package contents and
parts-tag replacement/reload regression checks were added or updated but
not run. Brief static review only; no builds, CI inspection, version checks
or merges. Actual acquisition, casting, inventory events, notifications,
save/reload and LAN remain unverified.
