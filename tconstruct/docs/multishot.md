# Multishot crossbow loading

This bounded port makes the pinned TConstruct Multishot ability obtainable through
ordinary Station/Anvil recipe processing and loads real arrows into a TConstruct
crossbow. The source recipe consumes two pistons, one steel ingot, two ichor slime
balls and one ability slot per level. Modifier crystals remain subject to the
existing source recipe permission and slot checks. Extraction uses the existing
modifier selection and ability-slot recalculation paths.

Pinned source: TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`BowAmmoModifierHook.consumeAmmo`, `ModifiableCrossbowItem`, and the bundled
`tools/modifiers/ability/multishot` recipe and `tinkering/modifiers/multishot`
definition. Desired count is `1 + 2 * effective level`, including material traits.
Unlike vanilla Multishot, every survival projectile consumes an arrow. The first
selected stack is consumed first, followed by matching item/component stacks;
insufficient ammunition produces a partial load. Different tipped-arrow contents
are never combined. Creative loading consumes nothing and marks arrows intangible.
The existing crossbow firing path supplies source fan angles and the middle primary
projectile. Charged projectiles use the ordinary ItemStack representation.

The final ready-to-load tick is intercepted on the authoritative host. All bow
and ammunition slots are rechecked after the native ammunition-provider call and
before any inventory mutation. Unknown virtual survival ammunition and rockets
are refused rather than duplicated. Vanilla crossbows and unmodified TConstruct
crossbows retain the existing path. Remote clients do not mutate inventory here.
The normal host charge clock, sounds and successful-load pitch RNG are preserved.

Acquisition and loading inspect the current modifier snapshot each time. Missing,
malformed or changed Multishot definitions fail closed. Reloading the exact source
definition restores eligibility. Malformed saved levels and unresolved material
traits cannot produce a paid acquisition. Effective levels 1–31 (3–63 arrows)
are supported; larger levels are refused, rather than silently truncated.

## Validation and remaining limits

- Pinned recipe/definition equality and cancellable hook ownership tests.
- Real host ItemStack/NBT tests for matching stacks, partial loads, creative
  flags, malformed levels, slots, ingredients and repeated ability cost.
- An adapter fixture compiles the unchanged production loading injection and
  tests successful atomic consumption, replaced bow/ammunition, snapshot reload,
  unsupported providers/rockets and trait bounds. It is not the full Game.
- Full pinned-original Mixins planning resolves the actual crossbow target.
- CI applies the full mod set, compiles the real client and runs the named
  Multishot and existing upgrade tests. A named-result assertion prevents a
  zero-test success.

Run the adapter with `python3 tconstruct/tests/run_multishot.py --horizon PATH`.
Host dependencies must already be fetched because this runner uses Cargo offline.
Full-client compilation, live gameplay/reload, save/network round trips, and LAN
have not been locally verified. Modifier-provided ammunition, rockets and source
levels above 31 remain outside this port. No broad #379 completion claim follows
from the bounded arrow-loading implementation.
