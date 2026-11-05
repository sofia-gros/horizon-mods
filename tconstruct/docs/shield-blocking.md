# Source shield blocking

Implements the ordinary right-click blocking runtime for `travelers_shield`,
`plate_shield` and `battlesign`, for issue #379. This is independent of the
unpublished armor-protection and Overshield damage-stage work.

## Covered

- Both hands enter native held use, with its cooldown, release and 72000-tick
  lifecycle. Broken or malformed authoritative tools cannot start protection.
- Blocking starts after five ticks. Live source definitions supply flat damage
  caps and full block cones: Travelers 10/90 degrees, Plate 100/180 degrees,
  Battlesign 50/120 degrees. The cone uses the pitched view and horizontal source
  direction, matching `InteractionHandler`; absent or coincident sources fail.
- Positive live Blocking is required. Blockade's supported source definition
  multiplies base block amount by `1 + level`, capped at 2048. Traits and saved
  upgrades are combined once. This does not add a Blockade acquisition recipe.
- Native damage-type bypass and piercing rules remain, as do nonbreaking-hit
  sounds, partial-hit knockback, projectile handling and axe disabling.
- Damage actually blocked, not incoming damage, costs `1 + floor(blocked)` wear
  from three blocked damage upward. Existing Overslime, Tanned and durability
  hooks run before the result is committed to the same unchanged held stack.
  Creative skips wear. Broken tools remain in their slot, stop use and play the
  shield-break sound, instead of native static durability deleting the stack.
- Stock source base use speeds are registered as 0.8 for Travelers and 0.2 for
  Plate/Battlesign, for both host and remote held-use slowdown.
- Source item tags, material/definition snapshots and canonical modifier JSON
  are checked live. Unknown extra tool-definition stat machinery, unsupported
  wire data and explicit stack `blocks_attacks` overrides fail closed.

## Integration

Both Python and native resource preparation register `blocks_attacks` and use
speed. The static component deliberately blocks zero damage and spends zero
native durability: only the validated source runtime grants protection and
wear. No new per-stack wire component or host extension is needed. Regenerate
prepared resources to pick up these item defaults.

The three injections touch only shield entry points, not the armor/EPF health
stage. Integrate Unburdened and other conditional-use-speed work separately;
this change only supplies the native stock base defaults.

Remote clients predict only stock held-use visuals/speed; the host resolves
protection and wear using its current data. Custom pack use-speed changes are not implemented by this change; a proposed
local hook was removed during static review because its native closure context
is unsupported by the current mixin parser. No mob-held blocking, generic Blocking or
Parrying on other tools, shield-specific modifier hooks (Reflecting, Springy,
Rebounding, etc.), source break-event API, custom shield models/poses, or
Overshield protection-point consumption is claimed.

## Source and review

Pinned source: SlimeKnights/TinkersConstruct commit
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, `BlockingModule`,
`InteractionHandler.onBlock/canBlock`, `ToolStats`, and bundled shield tool and
Blockade modifier definitions. Three pure tests are included but unrun.

Only brief static review, Rust formatting and diff whitespace inspection were
performed. No tests, builds, planner, CI, version checks or merges were run.
