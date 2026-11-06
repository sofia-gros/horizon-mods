# Item display language fixture: verified result

Status: the bounded native fixture passed on 2026-11-06. This is a result record, not a change to production code or the frozen preparation inputs.

## Exact evidence

- [CI run 42181415662](https://github.com/sofia-gros/horizon-mods/actions/runs/42181415662), [job 127771667512](https://github.com/sofia-gros/horizon-mods/actions/runs/42181415662/job/127771667512), workflow head `0e4d68ea9c0d9cdecf07aed14daf5dacc57710c5`.
- Fixture source `df0645ec2bdd1063364eb82d85c45ef45d61c876`; composed language sources `ea7095d80b67ab8659b26b3e0e32a92d77de2cae`.
- Official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` only. No fork source was used.
- Actual full, unfiltered suite: **22 passed, 0 failed, 0 ignored, 0 measured, 0 filtered**. One run partitions into item display 8, author Key 6, and internal language 8. The item group is not a second execution.
- The actual official core/items/world crates and test crate compiled. The unchanged author example type-checked through an explicit fixture-root `horizon_client` alias; this is not installed client-library type acceptance.
- Source guards 25/25 and runner guards 15/15 passed. Exact native dependency paths and lock identities, source/fixture/generated-output seals and final verification passed. Minimum free disk space was 90025824256 bytes, above the 10871635968-byte floor.

The individual eight item test lines, the strict whole-suite summary and `verify-after` exit 0 were read in the actual job log, in addition to `FINAL_RECEIPT passed=true, verified=true`. The adjacent [JSON result](item-display-language-result-20261106.json) records the exact names and seals.

## What the item eight prove

1. Standard `item.namespace.path` names use the selected language, then the native block-key and English/path fallbacks. Optional missing lookup remains distinct from displaying the key.
2. Slash paths and the implicit Minecraft namespace keep native behavior.
3. The same explicit translatable `minecraft:item_name` Tag resolves again in English and Japanese without mutating its saved component.
4. Custom names win over item names and retain explicit italic/color styling.
5. The author Key's retained translate/with/style component survives the tested native ItemStack NBT encode/decode path and resolves in a newly selected language.
6. Missing translation keys and explicit native fallback templates retain their different behaviors.
7. The actual `styled_name` function calls the current default-name lookup for an unadorned stack.
8. `ItemStack::new` does not fabricate an item-name patch. Supplying an explicit component is a separate case.

The fixture compiles the byte-exact official `Names` struct/implementation and `styled_name` function. It uses the real native Language, Text, ItemStack and component conversion implementations, not modeled substitutes. The pre-execution RGB expectation correction is retained in the original input record; this CI run needed no runtime correction.

## Boundaries still open

This result does **not** prove a live Game screen, all default-component construction paths, full installed native-package composition, general opaque/Wire or raw NBT losslessness, network delivery, recipe-view refresh or title-routing migration. The Game `stack_hover_name` and content `Plan::items` bodies remain source-hash/order evidence only.

For a bare stack, the existing standard item-key route is proven by the bounded fixture. For a nonstandard translated name, only the tested explicit item-name component path is proven. Declaring a content default does not, by this result alone, prove it is materialized in every stack creation route. No automatic materialization hook, new key registry, locale owner or TConstruct activation was added.

The [proof plan](item-display-language-proof-plan-20261106.md) and [input record](item-display-language-inputs-20261106.json) remain frozen historical preparation records. Their pre-execution status is not rewritten. Remaining title-routing, missing container keys and format/cache concerns stay in the separate i18n audit.
