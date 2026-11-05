# Chrysophilite ore loot

Related to #379. Stacks on worn Fortune #802 (and tool Fortune #796).

## Implemented

- Player block loot for source `tconstruct:chrysophilite_ores`, including the normal and area-break paths that call the same native loot method. No TiC harvesting tool is required for Chrysophilite itself.
- At least one correctly slotted stock worn armor item with canonical Chrysophilite activates one wearer-wide value, `1 + gold-neutral armor count`. Modifier levels and multiple copies do not multiply the attribute. Broken armor retains equipment-change effects and volatile flags, matching the source.
- Add 2–6 gold nuggets, then the source ore-drop multiplier with level equal to the gold-neutral armor count. No Fortune level is added to the Chrysophilite multiplier. Source ore items in the already generated loot suppress the bonus before any extra random draw, including ordinary Silk Touch ore drops.
- Gold-neutral counts include native piglin-safe armor, Golden, Gold Guard level 2+, and Chrysophilite level 2+. The latter also feeds the existing piglin-neutral and Gold Guard health paths.
- Preserve native table choice, dynamic drops, named random sequence or world RNG, and native loot callback. Generate the normal loot once, then append the bonus in the same RNG context. The real inventory item is unchanged.
- Preserve the original Chrysophilite global-loot JSON and Forge registration list in packaged and prepared source layers. Capture enabled datapack overrides separately from modifier definitions. Honor global `replace` and require the canonical highest-priority loot and equipment definitions. Unknown/malformed relevant resources fail closed. Parked snapshots are invalidated on reload.

## Bounded scope

This is not the whole Forge global-loot framework. Other registered modifiers and their relative interactions are not implemented here. No generic custom Chrysophilite attribute registration or command/third-party attribute changes, non-stock TiC armor, arbitrary native piglin-neutral item hooks, mob equipment-drop event, explosion loot context, or entity loot is implemented. The source mob equipment bonus reads the dying entity's attribute, not the killer's, and remains separate.

The crop adapter uses the same bounded modifier only when its actual block matches the source ore tag; normal crops remain unaffected. Source armor Fortune still stays out of that adapter, as in #802. Other native hook injections into the cancelled block-loot method need integration review alongside #796/#802.

## Review

Brief source/static review only. Added unexecuted pure tests for random draw bounds/order, ore multiplier, overridden definitions, and global-list replacement. No tests, build, planner, CI inspection, version checks, or merge were performed. In-world, reload, LAN, area mining, and Silk Touch validation remain for the final combined pass.

Source references: TinkersConstruct commit `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, `GoldenAttributeModule`, `ChrysophiliteLootCondition`, `ChrysophiliteBonusFunction`, and generated `data/tconstruct/loot_modifiers/chrysophilite_modifier.json`; Mantle `AddEntryLootModifier` applies post-conditions before entry generation.
