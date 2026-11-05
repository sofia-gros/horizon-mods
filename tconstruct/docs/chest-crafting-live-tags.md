# Chest crafting with live Forge tags

Part of high-priority #379. Part Chest and Tinker's Chest crafting now use their actual source ingredient tags:

- Both require one current `forge:chests/wooden` member.
- Part Chest requires four current `forge:rods/wooden` members.
- Tinker's Chest requires four current `forge:gems/lapis` members.
- Both retain one Pattern, one planks block, their existing three-by-three pattern and one-chest output.

The earlier native aliases combined literal vanilla defaults with Forge tag references. A datapack could add substitutes, but replacing or emptying the real tag could not remove the hardcoded chest/stick/lapis inputs. Direct tag references now follow the native current-world registry's merge/replacement semantics. Ordinary default-world ingredients remain unchanged.

The native content package now includes the exact pinned Forge defaults for wooden chests (Chest and Trapped Chest) and lapis gems (Lapis Lazuli). The existing native wooden-rods tag already supplies Stick and is unchanged. Old aliases remain present for compatibility with other consumers, but these two recipes no longer use them.

Recipe IDs, discovery criteria/rewards, item counts, result identity and chest behavior are unchanged. The source notification flag is explicit. Source ingredient symbol names remain mapped to the existing native `x` symbol without changing slots. No recipe matcher, crafting transaction, menu, storage or network code is modified.

Source recipes and Forge tag fixtures retain exact hashes and provenance in `tests/fixtures/chest_crafting_tags`; existing TConstruct and Forge license notices apply. Focused tests compare recipe adaptation, default tags, absence of fallback aliases, unchanged acquisition discovery and package paths. Tests were added but not run. Only brief static review and whitespace checking were performed; builds, CI inspection, version checks, crafting under replaced/empty tags, reload and LAN remain deferred.
