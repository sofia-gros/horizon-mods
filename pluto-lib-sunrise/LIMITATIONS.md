# PlutoLib for Sunrise — v1.0.0 limitations

PlutoLib v1.0.0 for Sunrise is a **reduced** release. PlutoLib's public
API is the same shape on every platform, and every function below is
present and callable — but on Sunrise the listed surfaces are **not
connected to the game yet**: calls succeed and register normally, yet
the in-game effect does not happen. They will light up in later
PlutoLib releases as the Sunrise mod API grows (most are already filed
with the Sunrise project, and several were implemented upstream within
a day of filing); mods built against v1.0.0 need no changes to benefit.
The full per-function validity list lives in the PlutoLib API docs;
this page is the feature-level summary.

## What is not connected in v1.0.0 (Sunrise)

**Creatures and combat**

- Attribute modifiers applied when a creature joins the world, and
  the attribute operation set (refresh/value) behind them.
- Mob effects: applying effects on tick, and effect icons.
- The mob-split dressing (extra children of a split mob lose their
  equipment/state dressing) and the projectile-impact game hooks.
- Living-entity drops captured on the game side, the death-loot head
  gate, and entity hurt handling on the game side.
- Fall damage and landing: the fall/landing state machines, block
  fall contact, inside-motion and after-fall handling.
- Entity spawn finalisation (spawners, raids, structure spawns),
  spawn-placement consultation, entity seating of players, entity
  teleport and particle access, and the entity-particle event kind.

**World and blocks**

- Structure generation and placement, tree placement, and the
  structure command — Sunrise's world generation is laid out
  differently and this family is being re-ported.
- Block placement context on the game side, block path-node queries,
  swept-inside contact events, contact sounds and contact effects,
  movement restitution for blocks and mobs, and block facts
  publishing / block-entity revision notices.
- Fluid rendering assembly hook and fluid use on the game side.
- Cloud creation and insertion into the world.
- The world-generation level surface (configured features access).

**Items and inventories**

- Item use start/finish dispatch (including the tool-use path),
  item wear reporting, shear-by-saddle, throw sounds, item colours
  (tint layers), item particles, and the item-placement export.
- Stack snapshot comparison, the stack store/storage pair, and
  wire-format reading of stacks (item comparison against a retained
  baseline is unavailable).
- Inventory advancement components, and the component view/display
  surfaces for stacks.
- The crafting-match adapter, the recipe-index patch, advancement
  projection and advancement tag indexing/predicates.
- Saved books (capture and the book itself), the guide-text surface,
  and the current-book guide adapter.
- The station ingredient parser.

**Menus and screens**

- Menu equipment slots (including guest loading), the menu-switch
  implementation and its flushes, and dynamic menu slot positions
  and their data updates.
- The creative screen's click handling and page preparation.
- Living-entity preview and map face/tick rendering flushes.
- Hanging frames and pictures: the game-side frame handling,
  explosion interaction for frames, and frame tick effects.
- Fishing on the game side, brush flushing, sleeves takeover,
  the sleeves' reload integration, and the explosion flush
  (explosions can be queued through the access API but the flush
  that detonates them is not connected — queued explosions,
  including projectile-flight ones, do not go off).
- Reload listener registration around data-table loading.
- The protection contribution/facts surfaces on the player side.

## Mod-specific limitations in this release set

- **Classic Pipes (Sunrise v1.0.0):** a broken pipe does **not** spill
  the items it was carrying. PlutoLib's Sunrise block-entity API has
  no removal hook yet (the Horizon edition has one); the spill
  returns when the API lands, with no change needed in the mod.
- **Classic Pipes (Sunrise v1.0.0):** the pipes have **no creative
  tab** yet. All four pipes are fully craftable in survival, but they
  cannot be taken from a creative inventory tab. PlutoLib's
  creative-tab registration reach is not connected on Sunrise yet
  (filed upstream as sunrise-org/sunrise#5950); the tab appears when
  it lands, with no change needed in the mod.
- **Stellarsky (Sunrise v1.0.0):** the three **static sky texture
  layers** — the faint star sphere with the Milky Way and its dark
  bands (magnitudes 4–7), the airglow band, and the zodiacal light —
  are **not shown**. PlutoLib's Sunrise sky surface has no
  texture-layer registry yet. The dynamic layer works in full: the
  bright stars drawn star by star with their twinkling, the
  Sagittarius A* dark patch, and the suppression of the vanilla
  star field.

Everything not listed here — including the whole content API, the
hooks bridge, fluid storage and transport, block entities, menus,
recipes, loot, and the dynamic sky — is connected and works as
documented.
