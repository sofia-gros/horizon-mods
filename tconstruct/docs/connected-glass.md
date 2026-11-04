# Connected glass

> **Provisional build:** Some effects and operations described below are unavailable or under verification. Earlier test results refer to their stated revisions; see [current limitations](provisional.md).

Clear, colored, seared, and scorched glass and panes are available in the **Tinkers' Construct Blocks** Creative tab. Connected world-face textures and stained world-face tint are currently unavailable in this build; see the integration limitation below.

![Clear glass joined next to colored and seared glass](connected-glass.png)

*Earlier implementation: the two clear blocks on the right joined into one surface. The current build does not implement connected world-face textures.*

Panes are intended to change shape with their neighbors and keep exposed edges where stacked connections differ. These behaviors require verification on the current build.

![A pane edge visible beside a narrower pane above it](connected-pane.png)

*Earlier implementation: the lower pane has an arm that the pane above it does not, so its exposed top remains visible.*

Soul Glass can be walked through and slows movement. Earlier Linux gameplay checks applied to a prior implementation, not the current build. Connected appearance, pane shapes, breaking, saving, LAN synchronization, Soul Glass view-blocking behavior, and Windows play need verification on the current build.

Smeltery behavior, material-dependent anvils, and custom storage are separate work. See the [block guide](block-registration.md) for the rest of the registered building blocks.

## Clear glass panes

The pinned source recipe crafts 16 Clear Glass Panes from six Clear Glass blocks in two rows of three. Source, registration and recipe-collision checks pass. The source data also contains a sand-to-molten-glass and basin-casting route to the input, but actual acquisition, crafting, recipe-book display, save/reload and LAN remain unverified.

## Stained clear glass

The pinned source recipes dye eight Clear Glass blocks with an item from the matching `forge:dyes/<color>` tag to produce eight colored blocks. The sixteen additive dye tags include their Forge 47.4.0 defaults. Source, registration, collision and package-content checks pass. The host's existing Forge namespace ownership restriction and lack of Forge tag `remove` support limit cross-mod and datapack parity. Actual acquisition, crafting, recipe-book display, save/reload and LAN remain unverified.

## Stained clear glass panes

For each of the 16 colors, the pinned source recipe makes 16 matching stained Clear Glass Panes from six matching stained Clear Glass blocks in two rows of three. Source, registration and collision tests pass. The matching stained-glass recipes provide the input at the data level. Actual acquisition, crafting, recipe-book display, save/reload and LAN remain unverified.

## Direct pane dyeing

The pinned source also dyes eight uncolored Clear Glass Panes with a matching tagged dye into eight colored panes. All 16 colors are represented. Source and registration tests pass; existing Forge namespace ownership and `remove` limitations apply. Actual acquisition, crafting, recipe-book display, save/reload and LAN remain unverified.

## Current rendering integration limit

The source sprites are added to the atlas and pane face visibility has a hook, but no active mesh hook selects connected sprites or applies stained world-face tint. The older `connected-mesh-face.rs` helper is not registered and must not be enabled unchanged: it replaces the face path and does not preserve the pinned host's current Fusion and OptiFine picks, shader lighting, ambient occlusion, overlays, and quad routing.

At Horizon `0c7b5b61` and Mixins `ddd9700`, a narrow `MODIFY_VARIABLE` hook on the face's original tile or tint fails planning with `E_INVALID_BINDING: destructured function parameters are unsupported`: `Mesher::face` has the parameter `(id, state): (u32, &Info)`. A compatible host or Mixins hook must expose the face context without replacing the full renderer. Any replacement needs tests for source connection and pane predicates, tint, host rendering behavior, other mods, and in-game verification.
