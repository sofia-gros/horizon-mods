# Tanned live durability causes

Local high-priority #379 slice. Based on the unpublished Consecrated branch.
Publication remains paused.

## Implemented

- Replace the fixed secondary-cause name list with Tanned's two current source
  range predicates. Non-bypassed causes clamp wear to one; secondary causes
  suppress it, including when they also bypass the first range. Ordinary wear
  uses the source `tconstruct:empty` cause, which packs can tag explicitly.
- Read the highest-priority Tanned definition and current modifier-tag snapshot
  on every supported wear operation. Nested tags, additions, replacements and
  removals are respected; nothing is cached across resource reloads.
- Resolve material traits plus saved integer Tanned levels. Reject malformed,
  broken, exhausted, stacked or opaque-wire stacks rather than applying a
  guessed bonus.
- Connect this evaluator to the existing active `damage_with_cause` path before
  Overslime is spent. The retained ordinary-armor helper uses the same evaluator,
  eliminating the stale table there as well.

## Registry boundary

This is a deliberately bounded reachable-tag resolver, not a complete modifier
registry. Every referenced leaf must be proved canonical and registered: the
source static Empty/Enderporting entries, one of the eleven exact retained
dynamic definitions used by the stock secondary tag, exact Tanned, or the
existing supported module-free definition shapes. Unknown optional leaves are
not assumed absent. Redirects, unsupported changed definitions, unresolved
required nested tags and cycles disable this Tanned reduction for the operation.
The input wear continues through the existing downstream chain in that case.

Only tag graphs reached from the two relevant roots are examined. An absent
root is an empty tag. Invalid individual JSON layers follow the existing tag
parser's whole-file rejection; an entirely invalid reachable tag is unsupported.
The embedded source definitions prove registration, not membership: membership
comes exclusively from current tag layers. No alias-object removals are guessed.

## Not completed

The ordinary `hurt_armor` record/end and generic Dense durability injections
remain disabled in `disabled-injections.toml`. This does **not** restore normal
armor wear, Dense/Reinforced ordering, or the whole grouped issue row. It also
does not add Tasty armor counters. Inspection found that Tasty's source
`ON_ATTACKED` callback eats before shielding/cooldown/ordinary armor wear; the
current native pre-hit environment cannot update Game equipment and food
together. No post-hit approximation was added.

## Review and deferred validation

Brief static review covered both call sites, two-range precedence, resource
replacement, canonical identity/removal restrictions and fail-closed behavior.
Three regression cases were added and the old range assertions were updated.
They were not run. `git diff --check` only; no tests, builds, planner runs, CI,
version checks or merge. Gameplay, save/reload and LAN remain unverified.

Reference: Tinkers' Construct MIT source at
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`:

- `src/generated/resources/data/tconstruct/tinkering/modifiers/tanned.json`
- `src/generated/resources/data/tconstruct/tinkering/tags/modifiers/durability/{bypass_tanned,secondary}.json`
- `ToolDamageRangeModule`, `ToolDamageUtil`, `TagModifierPredicate`,
  `ModifierManager`, and static registrations in `TinkerModifiers`.
- The eleven embedded JSON definitions are the source secondary-tag members
  other than statically registered Enderporting, under the same modifiers path.
