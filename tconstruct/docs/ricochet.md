# Ricochet: bounded received-knockback multiplier

Related to #379. Source reference: Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, `ricochet.json`,
`AttributeModule`, `ModifiableArmorItem`, and `ModifierEvents.onKnockback`.

## Connected behavior

- Live source traits plus saved upgrades on intact, correct-slot stock
  Travelers/Plate/slime armor and selected-mainhand/offhand stock shields
  contribute Ricochet. Held scope is Travelers Shield, Plate Shield and
  Battlesign. The selected inventory slot is used, not a temporary acting hand.
- Source `MULTIPLY_BASE` amounts add: starting from one, each eligible piece
  contributes its individually float-evaluated `0.2f * level`, widened to
  double. Two level-one pieces therefore make a 1.4 multiplier, not 1.44.
  The resulting source multiplier is clamped to 0..100.
- This increases knockback received, not resistance or damage dealt. Before
  the native player's knockback routine, multiply incoming strength and cast
  once to float, matching the explicit cast in ModifierEvents. With no effect,
  the input strength is unchanged, including its original precision.
- Native cancellation, knockback resistance, direction randomization, velocity
  halving, vertical-motion rules and the vertical cap remain in the host.
  Crystalstrike's direction adjustment changes separate arguments and can
  compose with this strength multiplier.
- Ricochet uses default `tooltip_style=attribute`, unlike Crystalstrike's
  percent equipment hooks. Broken tools/armor therefore contribute nothing.
  No shield needs to be actively blocking. The module has explicit hand slots,
  with no `held_tag` predicate for its attribute.
- Each received hit resolves source definitions, tags, traits, upgrade levels,
  durability and both hands afresh. Unequipping, repairing, breaking, same-item
  material edits and definition reloads need no persistent cache cleanup.
  Missing/modified definitions, opaque stacks and malformed levels fail closed.

## Sky Slime status contribution

The registered `tconstruct:ricochet` status is neutral (`0x01CBCD`). Sky Slime
contact supplies 100 ticks at amplifier zero, protected from generic curatives.
The existing received-knockback adapter reads the live effect at every hit.
Its `0.5 * (amplifier + 1)` MULTIPLY_BASE contribution adds to the equipment
factor before the final source float cast; it does not multiply that factor.
The combined total is clamped to 0..100. Expiry and removal need no cache reset.
If equipment data is unavailable, its contribution stays excluded while the
independently known effect still applies. Pure regression cases for level,
composition, clamping and removal were added but deliberately not run.

## Remaining boundaries

Only knockback routed through `Game::knock_back_player` is connected. Other
native/direct motion paths, mobs wearing/holding TiC items, staff/other held
item classes, shield blocking mechanics and acquisition changes are not covered.

The custom source knockback-multiplier attribute is not registered here.
External attribute modifications and the separate legacy TinkerData knockback
contribution are not combined. The adapter runs before native callbacks;
full cross-mod callback and source UUID attribute-order parity is not claimed.

Actual combat, mixed Ricochet/Crystalstrike, breakage/repair, save/reload,
remote prediction and LAN have not been exercised. Three pure regression
tests were added but not run. Brief static review and formatting only; no tests,
builds, planner runs, CI inspection, version checks or merges were performed.
