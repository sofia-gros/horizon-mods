# Maintained armor toughness

Local high-priority #379 implementation, stacked on Tanned live wear causes.
Publication remains paused.

## Added behavior

- Require the current canonical Maintained definition and the live
  `tconstruct:modifiable/armor` tag. Resolve material traits plus saved integer
  modifier levels on the 13 supported worn armor items, in their proper slots,
  and the three supported held shields in either hand. Held candidates also
  require the live held tag.
- Add toughness per equipped piece using the source float evaluation order:
  `max(current_durability - max_durability * 0.5, 0) / (max_durability * 0.5)
  * level * 2`. At an integral maximum this gives two toughness per level when
  undamaged, one per level at 75% remaining, and zero at 50% or less.
- Preserve the distinction between the float maximum stat and integer current
  durability. The current supported material/gem rebuild baseline provides the
  float denominator. Its integer truncation must match the saved maximum;
  otherwise the bonus is disabled rather than guessing after a stale rebuild.
- Keep the bonus in six owned transient attribute IDs. Replace or remove only
  these IDs, preserving base toughness and external modifiers. Unchanged values
  do not cause repeated attribute updates. Broken, exhausted, invalid, removed
  or newly ineligible equipment clears the bonus. Remote clients retain the
  server-provided attribute map without local removal or recomputation.

## Refresh boundaries

The current actor's equipment and status refreshes, resource reload, and native
player-tick completion update the projection. Additional hooks refresh at the
start of the local tick; before `hurt_player_from` and `hurt_player_by`; after
`wear_armor` finishes; and inside the guest actor context before its server tick.
The guest refresh explicitly reads the offhand as well as same-ID component and
wear changes, independent of the native equipment cache's missing offhand entry.
No actor-indexed global bonus cache is used.

## Scope limits

This is Maintained's toughness attribute only. Mining speed and launcher velocity
are unchanged. General dynamic armor wear is still disabled upstream in this
branch; this change supplies a safe completion refresh for when wear is applied,
but does not claim to restore that separate pipeline. The native engine may
queue multiple armor-damage events before committing equipment wear; this work
does not change that event order or promise per-hit wear inside that batch.

Raw durability uses the existing supported rebuild formulas and modifier order.
Unsupported saved Overlord upgrades and inconsistent saved maxima fail closed.
Arbitrary held tools, non-player equipment and unknown item classes are outside
this bounded adapter. Full transformed compilation, gameplay, prediction,
save/reload and multi-guest LAN remain unverified.

## Review

Brief static review covered the source formula, fractional maximum, broken
guards, owned-attribute replacement/removal, remote behavior and actor context.
Three regression cases were added, not run. `git diff --check` only. No tests,
builds, planner runs, CI inspection, version checks or merge.

Reference: Tinkers' Construct MIT source at
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, generated `maintained.json`,
`AttributeModule`, `ToolVariable`, `ToolStatVariable`, `IntegerToolStat` and
`StatsNBT.getInt`. No native dependency or provenance-blocked movement work is
used by this implementation.
