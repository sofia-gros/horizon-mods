# Wings acquisition

This #379 slice adds the source Wings ability acquisition. It does not enable
TiC flight, and the workstation description explicitly states that flight and
gliding wear are not connected yet.

One Elytra, or a matching Modifier Crystal, adds the single purchased Wings
level at an existing workstation. The source recipe checks material traits, so
an item already receiving Wings from its definition/material cannot acquire a
redundant purchased level. Only known Travelers/Plate/Slimy chestplates and
Slime Wings admitted by the current chestplate tag are considered. Slime Wings'
ordinary built-in trait is therefore rejected by the duplication check.

Wings costs **two ability slots**, not one ability or any upgrade slots. The
shared capacity accounting now weights Wings correctly after application,
material swaps and other ability additions. Extraction removes its one saved
level and releases both occupied ability slots. Existing extraction selection
indices are unchanged. Malformed or out-of-range saved Wings values are
rejected before evaluating slot arithmetic.

The existing station transaction, assembled-tool rebuild, custom-data upgrade
map, crystal creation and sponge extraction remain in use. The exact source
recipe and highest Wings modifier definition are required. The source flag is
spelled `tconstruct:elyta`; this slice does not silently change it to `elytra`.
Changed cost, trait checking or module contracts fail closed.

## Native flight boundary

The host already has a component-based glide entry point, server guest checks
and once-per-second glider wear. However, `item_sync::can_glide_using` checks the
item registry's static maximum durability, while TiC armor stores a computed
maximum and its own broken state. `Game::wear_glider` writes damage directly to
a selected glider. Adding a persistent glider component alone would bypass the
needed TiC durability/broken-state and source wear handling. No such component
or shortcut is installed here.

A later runtime slice must connect eligibility, wear, break/repair transitions,
removal, client prediction and server validation together. Rendering, flight
modifier hooks, Overslime/Reinforced behavior, save/reload and LAN are not
claimed. Acquisition alone does not complete the Wings high-priority item.

Brief static review covered source input/trait checks, two-slot accounting,
extraction, saved-value validation and the native flight boundary. Regression
cases were added but not run. No tests, builds, Mixins transforms, CI inspection,
version checks or merge were performed. Publication is paused. This branch
stacks on Double Jump acquisition, which itself stacks on Depth Strider.

Sources: Tinkers' Construct f4e77d43,
`recipes/tools/modifiers/ability/wings.json`, `tinkering/modifiers/wings.json`
and `ModifiableArmorItem.java` (`canElytraFly` / `elytraFlightTick`).
