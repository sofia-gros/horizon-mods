# Double Jump worn-armor runtime

High-priority issue #379 armor-upgrade effect slice. This stacks on Dreambound
jump bridge #842. Normal boots acquisition, levels one/two, ability-slot cost,
crystals and extraction are separately published in #887; those recipe changes
are not copied into this branch.

- Resolve saved Double Jump upgrades plus live tool/material traits on stock
  worn armor in its correct slot. Require one intact item, live modifiable and
  worn tags, valid data and the canonical level-scaled source definition.
- Add one extra jump per Double Jump logical level. The usual boots levels one
  and two grant one and two airborne jumps. The source module allows all worn
  slots, even though the ordinary acquisition recipe targets boots.
- Combine this with Dreambound's flat one jump per eligible piece before
  spending from the existing shared airborne budget. Four Dreambound pieces
  and Double Jump II boots therefore grant six extra jumps, not separate
  counters that could replenish one another.
- Match TinkerAttributes.JUMP_COUNT's base/minimum one and maximum 100 by
  clamping the extra allowance to 99. Extend both the existing wire validation
  and native movement guard from the former four-piece-only limit to 99.
- A positive modifier with a missing/replaced definition fails closed. A missing
  unrelated definition does not disable the other valid modifier. Broken or
  incorrectly slotted pieces contribute nothing.

The existing native jump call, fresh-press handling, ground/landing reset,
equipment-bound permission frame, authoritative guest re-evaluation and
no-refill-on-equipment/effect-refresh semantics are reused. There is no new
movement injection or second jump packet/budget. The payload structure and
channel remain unchanged; older bridge clients reject capacities above four.
Both ends should use the integrated implementation for the expanded range.

Existing limits remain: prediction-recording servers, replay, water/lava,
climbing, gliding, creative flight, riders, sleepers, dead players and
negative-gravity motion are gated off. No source jump sound/particles or saved
midair usage counter is added. Other jump-count effects, arbitrary native
attribute commands and non-stock armor are not included.

Source: SlimeKnights/TinkersConstruct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, generated
`tinkering/modifiers/double_jump.json`, `TinkerAttributes.JUMP_COUNT` and
`DoubleJumpHandler.extraJump`.

Two new pure rule tests were added, and the existing wire test was extended for
six/99 capacity and rejection above 99. None were run. Brief static review,
formatting and whitespace inspection only. No tests, build, planner, CI,
version checks or merge. Acquisition-to-movement, equipment transitions,
persistence, reload and LAN await the later combined pass. No paused branch
was incorporated.
