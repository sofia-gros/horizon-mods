# Latest crafting migration boundaries

Source-only comparison at official Horizon **41f822b659789780f58d32c63ddfb6887ea1b8bf** and Mixins **5a64af938ec86b00d53d16a408aabc0038fb3e57**. Mod input is exact main **08afa992d5091e03360f3473ea93fd17aa793987**. Ordinary Pluto input is the actual **4f13a6e2** archive (full hash and member hashes in evidence.json), not an excluded experiment directory. No ordinary manifest, API, hook, policy or guard was changed. No native command ran.

## Four callers require three different migration units

1. `anvil-material-recipe` is deny-only. In shaped recipes it first recognizes the crafting-station subtraction by output ID, dimensions, and the first two ingredient sets; presence of a crafting station in the actual cropped stacks denies. Its anvil branch reads the created result's stored `minecraft:custom_data/tconstruct:anvil/RequiredMaterial` as a native String, then requires exactly three fake-storage stacks with matching stored `Material`. Both strings use the existing lossy UTF-16 conversion. Absent/malformed expected material continues; absent/malformed input material denies. These details remain TConstruct policy.
2. `survival-melter-no-container` is deny-only. In shaped melter/fluid-cannon recipes it checks actual gauge input stacks. A missing `custom_data/tconstruct:tank` permits native matching; **any present Tag**, including an empty or zero-amount compound and a wrongly typed scalar, denies. It does not interpret fluid/amount or reject an unrelated input merely because it has other metadata. This policy remains local.
3. `overslime-crafting-assemble` supplies an actual output, and its separate remainder hook supplies a grid. The shared plan clones the tool, changes its owned custom data, and chooses consumed glass bottles versus returned original bottles. These are not denials. The existing assembly caller compresses cropped nonempty stacks, whereas the remainder caller supplies the original grid. Preserving the same pair and each coordinate mapping requires an authoritative consumption contract; two independently recomputed callbacks are not automatically the same invocation. Pluto's synchronous pair-staging API does not itself supply this automatic authority.
4. `crafting-station-recipe-placement-guard` owns a menu transaction: it validates the local station and nine-slot block entity, refreshes its grid, rejects the wrong menu/slot mapping or impossible placement, runs native placement, checks no drops and component-sensitive item-count conservation, then publishes the accepted grid and syncs the station. Ghost/UI changes follow. A deny-only placement callback cannot substitute these commits, station persistence, menu rejection or slot mapping. Its actor/remote behavior must be preserved explicitly.

The ordinary matching/placement complete-plan guards reserve their boundaries. The old hooks cannot simply remain alongside them, and deleting consumers or relaxing the guards does not prove migration. These are source-established intersections, not a newly accepted full-plan result.

## Exact latest native behavior to retain

`crafting.rs:1806–1845` still separates Special, Shaped and Shapeless matching. The normal path checks native geometry/ingredient predicates before calling `result.create()`. `recipe_matches` also uses this same function. `Input::stacks` (659) flattens cells in row-major order. `remainders` (1909 onward) independently reconstructs original positions from left/top/width and applies native item remainders and special-copy rules.

`Template::create` (304) clones the stored item/count/damage/extra. It does not apply the new `components` patch through `Template::over`. The proposed anvil facts must come from the same created stack as the old policy, not from raw template JSON or a reconstructed effective patch. Raw patch removals and zeroes must remain separate native concepts. No new interpretation of them is required for these two deny policies.

`recipe_book.rs:1013 onward` distinguishes remote packet submission from local menu processing. A local station transaction port therefore needs actor/remote/guest and menu-commit proof, not merely a successful read-only preview. The ordinary preview remains read-only and never grants item movement.

## Why existing ItemFacts is insufficient

The current ordinary adapter supplies shaped dimensions, ordered ingredient sets, cropped input cells and the created fixed-result stack view. Those facts cover the structural policy. Its `ItemFacts::capture`, however, intentionally sets `components=None` whenever recursive retained Wire exists. The old policies directly call `stack.data().and_then(|d| d.other("minecraft:custom_data"))`, even in that case. Treating unavailable components as absent changes both the material and tank rules.

Official `StackData::other` (item_data.rs:136) reads the existing native field: custom_name is special-cased; other IDs select the first matching stored pair. It does not expand prototype defaults or validate raw Wire. Preserve duplicate ordering, absence versus present malformed/empty values, and raw UTF-16 string units. A semantic saved-component projection is not a substitute for this narrow stored-value query.

## Smallest proposed generic extension (not implemented)

Keep the existing registry as the sole owner. Add an opt-in callback variant to that registry, using a separate extended request rather than adding required fields to existing public Request/Registration struct literals. Preserve old registration functions and their current facts. No second callback registry or TConstruct IDs/schema go into Pluto.

For the opted-in variant, the native adapter would lend a short-lived reader over the **same** cropped input stacks and the **same** created result stack. It exposes stored `other(component_id)` and borrowed nested-value operations: exact presence/type, compound key lookup, and raw UTF-16 string units. The public interface remains host-independent; its native implementation delegates to the actual Tag/StackData reads. The lifetime ends with the synchronous callback. No full tree clone, native-stack handle, write method, source-pointer identity, default expansion, rawpatch decoder, save claim or actor capability is provided.

Cheap-interest filtering remains before value capture. No-provider and uninterested paths continue native unchanged. Deny wins, callbacks run unlocked in the existing deterministic order, and Pass never creates an output. An unavailable capability must not silently masquerade as absent data. Current and new callback categories need explicit ordering under the single registry before implementation is accepted.

TConstruct can then express the two existing policies against this borrowed view, including the original UTF-16 conversion and all malformed cases. Registration must occur before matching is reachable, including recipe_matches queries. The exact startup composition must be proved before old hooks are removed. The other two authority units remain blocked until their own contracts are proven.

## Required evidence before integration

- Actual 41f ItemStack/Tag/Template inputs and real API dependency identity, not a stub Game or recreated NBT.
- Differential old/new two-policy results with no-Wire and recursively retained-Wire inputs; unknown fields and input/output source values unchanged.
- Expected-material absent, wrong type, empty string, UTF-16 edge cases; zero/two/three/four fake stacks, matching and mismatched materials, first duplicate stored key.
- Missing tank versus End/scalar/empty/zero/nonzero compound; irrelevant gauge/output/recipe kinds; holes/order, crafted-station subtraction and shaped dimensions.
- Default/no-provider/native fallback; mixed old/new callback categories, duplicate registration, deterministic denial, no callback under lock and no component capture for uninterested policies.
- Accepted latest complete plan with old two hooks removed only after equivalent callbacks are wired; all matching guards still pass. Overslime remains explicitly unresolved rather than hidden by guard exceptions.
- Applied native adapter plus TConstruct consumer type-check, then actual matching behavior. Separate menu/actor/remote tests and same-pair consumption proof are still necessary for the transaction units.

This document is an integration plan and exact source ledger. It is not a production activation, an implemented API, a Rust result, or evidence that the complete combined game works.
