# Blockade incremental acquisition and shield strength

Issue #379 shield-functionality follow-up, stacked on stock shield runtime
PR #851. This adds the pinned source acquisition path; it does not add generic
Blocking or Parrying acquisition/runtime to other tools.

- The live canonical incremental recipe accepts Obsidian Panes, ten per full
  Blockade level, up to three upgrade levels. It requires both durability and
  interactable/charge tags, plus an existing Blocking or Parrying modifier
  from saved upgrades or current tool/material traits.
- Each newly begun level occupies one upgrade slot. Additional panes finish
  an already-started level without requiring another slot. One application
  consumes only enough panes to finish the current level, leaving the rest.
- A modifier crystal adds one full level with the same requirements, cap and
  slot checks. Extraction removes one complete level, preserves partial
  progress, returns its modifier crystal and dries the wet sponge. Extraction
  uses the existing generic modifiable/non-unsalvageable rule. Zero-count
  extraction ingredients and zero-count Blockade inputs are rejected.
- Saved `tic_upgrades/tconstruct:blockade` stores pane progress as an integer
  in 0..30, matching the existing incremental-upgrade storage convention.
  Malformed or over-limit values fail closed rather than being overwritten.
  The current material trait level is separate and is added only once.
- The shield runtime now uses `trait level + pane progress / 10` as a float.
  A Travelers' Shield with five applied panes therefore blocks 15 damage,
  rather than treating five panes as five full levels. Plate and Battlesign
  reuse their existing base amounts and the source 2048 final cap.
- Station slot validation, rebuilt-tool requirements, selection, extraction,
  tooltips and Station progress display recognize Blockade. A material change
  cannot leave a saved Blockade upgrade without Blocking or Parrying.
- Before authoritative acquisition, the current modifier JSON must match its
  supported source definition and current material traits must resolve.
  This lookup works in both the host game and shared server namespace.

Source: SlimeKnights/TinkersConstruct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, generated
`recipes/tools/modifiers/upgrade/blockade.json`,
`tinkering/modifiers/blockade.json` and `StatBoostModule` effective-level math.
The bundled archives were inspected without running generation or tests.

The selection entry is appended, preserving this branch's earlier indexes.
Final integration must reconcile parallel appended entries such as Warping,
Worldbound and the locally held defense acquisition changes. No paused branch
was incorporated. Obsidian Pane production and other-tool Blocking/Parrying
remain outside this change; ordinary shield runtime remains bounded by #851.

Three pure acquisition tests were added and the existing shield rule test was
extended for fractional strength and nonfinite input. None were run. Only
brief static review, formatting of the changed shield files and whitespace
inspection were performed. No tests, build, planner, CI, version checks or
merge. Real Station/Anvil operations, persistence, reload and LAN await the
later combined pass.
