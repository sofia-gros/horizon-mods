# Retained armor durability writeback

Partial high-priority #379 repair, stacked on Maintained toughness #917. This
restores the ordinary damage-to-retained-item commit for a bounded subset of
stock armor. It does not claim the full source durability-modifier pipeline.

## Native boundary

The previous collector started in `Game::hurt_armor`, but its record and commit
injections were disabled because their targets contain unsupported macros.
The replacement wraps the single `hurt_armor` call inside macro-free
`Game::wear_armor` and records the final result of macro-free `Game::wear`.
Native damage selection, damage/4 flooring with minimum one, fire eligibility,
Unbreaking evaluation, and native item-damage callbacks remain in place.

Each `ArmorHurt` event gets a temporary actor-keyed collector. Its final native
wear is committed before the next event. The collector is removed before
attribute refresh; an outer collector is restored afterward. The old unclosed
HEAD collector is no longer installed. This also prevents unrelated later held
wear from being mistaken for an armor transaction.

Only exactly matching, still-equipped snapshots are written. More than one
matching callback or equipped copy is ambiguous and is not committed. Fixed
native-maximum equipment stays entirely native, preventing double damage.
Damage saturates at the saved positive maximum and marks the stack broken
without removing it. Break transitions refresh equipment; every changed record
refreshes Maintained toughness. No pending Overslime record is consumed here.

## Deliberately bounded eligibility

- Authoritative noncreative actors only. The acting guest's equipment is used;
  remote clients do not author this data.
- The 13 known stock worn armor IDs, in their class slot, with current armor and
  durability tags, count one, no opaque wire patch, positive saved maximum and
  valid unbroken state.
- Native enchantments, native/source unbreakability, positive or malformed
  stored Overslime, and malformed/negative modifier progress are rejected.
- Every positive material or saved modifier must match one of 14 pinned complete
  definitions: the five typed Protection upgrades, Iron Armor, Knockback
  Resistance, Maintained, Fortified, Heavy, Solid, Featherweight, Ductile and
  Preserved. These definitions have no tool-damage reduction hook. Changes,
  unknown modifiers and missing definitions fail closed for that piece.

This allows ordinary no-prevention armor combinations, including the canonical
Iron armor/mail material traits. It intentionally does not start charging
Tanned, Dense, Reinforced, Unbreakable, Solar Powered, Overslime, Barkskin or
other unsupported durability-hook combinations as though their prevention did
not exist. Those pieces keep the prior no-writeback behavior until their full
ordered pipeline is connected. Other safe modifiers can be admitted separately
with source evidence; this is not a general modifier interpreter.

Native batched health events remain batched: changing armor after an event does
not retroactively change damage already calculated for later events in that
native batch. Generic damage-change modifier callbacks, source break animation,
all source/native advancement ordering, custom armor and arbitrary reentrant
native mod callbacks are not implemented. Ambiguous reentrancy is refused rather
than guessed. Save/reload, dedicated server, guest/LAN, prediction and gameplay
remain unverified.

## Sources and review

TConstruct MIT source at
[f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2):
`ModifiableArmorItem`, `ToolDamageUtil.damage/directDamage`, material traits and
the 14 retained modifier JSON definitions. Canonical JSON retains its SlimeKnights
copyright and refers to the bundled MIT notice. No Forge implementation or
provenance-blocked native fluid code is used.

Brief static source/API, manifest-boundary and whitespace review only. Four
regression cases were added but not run. No tests, builds, planner, CI inspection,
version checks or merge were performed. This mod branch adds no native extension;
its base defense chain still depends on unpublished native signed-protection
09f0179db and protection-upper-bound 5e47547e2.
