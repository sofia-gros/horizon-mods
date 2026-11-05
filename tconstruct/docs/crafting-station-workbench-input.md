# Crafting Station workbench input

Part of high-priority #379. Restores the source recipe's workbench exclusion without changing the station's menu or crafting transactions.

The original recipe is one Pattern above one member of `forge:workbenches`, excluding `tconstruct:crafting_station`. The previous native alias included the Crafting Station and therefore allowed consuming a Pattern to recreate the same station. It also hardcoded the vanilla Crafting Table outside the live Forge tag, so replacing that tag could not remove the fallback.

The recipe now uses the host-supported `neoforge:difference` ingredient shape over the current `forge:workbenches` tag, subtracting the Crafting Station. It uses the existing native recursive ingredient loader and match-time difference check; no new runtime hook, transaction or item mutation path is introduced.

The native Forge tag is populated from the exact pinned source definition: Crafting Table, Crafting Station, and the optional legacy `forge:workbench` tag. Higher world packs retain normal tag merge/replacement precedence. The old `tconstruct:source_forge_workbenches` alias is left in place for other consumers, but this recipe no longer uses it.

The two-cell vertical pattern, one-station result, recipe ID and Pattern-based OR recipe-unlocked discovery remain unchanged. Source notification behavior is explicit. Log/table retexturing recipes, material components and other workstation acquisition routes remain outside this change.

Exact source recipe, advancement and tag fixtures/hashes are under `tests/fixtures/crafting_station_workbench`, covered by the existing TConstruct MIT license. Tests check exact adaptation, exclusion shape, source tag and unchanged discovery/package paths. Tests were added but not run. Only brief static review of the native difference parser and whitespace checking were performed; builds, CI inspection, version checks, actual crafting/recipe placement, reload and LAN remain deferred.
