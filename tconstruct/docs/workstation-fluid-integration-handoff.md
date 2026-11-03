# Workstation and fluid integration handoff

This candidate merges the fixed material integration #309 (`0c04d101a604374be7a068b933bcfdc45a227f37`) into the fixed workstation/fluid candidate #320 (`4e24f01322838a16af95755133d6d3af782a65d8`). These are both merge parents; their fixed heads remain unchanged.

| Workstation / fluid series | Adoption in this candidate | Verification boundary |
| --- | --- | --- |
| #298, #302, #303, #305 | Inherited through #320: unified splash aliases, restricted recipe and core effects, bounded guest cures and guest transaction | Apply, type and game results belong to their exact tested heads; inherited code is not a new runtime pass |
| #308, #313, #315, #317 | Inherited through #320: splash and lingering bottle registration/brewing, source/cauldron use, conflict guard and waterlogged selection | Natural acquisition, cauldron and world interactions require official target tests |
| #310 / #268 | Named `Ingredient.items` in `anvil-material-recipe.rs` | Required by newer Horizon ingredient shape; official type-check of this combined candidate remains pending |
| #314 | Equivalent ordinary-comment header correction inherited via #315 | #314's fixed head remains separate |
| #320 | Header static checker and historical audit evidence | Run checker against the selected final candidate and new changes |
| #309 | Material/armor and upgrade logic merged with fluid series | Combined candidate requires dedicated apply, type, behavior, and package verification |

The one textual conflict was `source-upgrade-rules.rs`: this candidate retains #309's speedy/step-up and gilded slot and level handling together with splashing's allowed tools, ability accounting, fluid levels and recipe handling. Other automatic merges need the same formal review and validation as the full candidate. No claim of completion or gameplay pass is implied.

## Later material merge candidate

A dependent candidate merges this fixed #323 head with material #324, carrying
#311 onward, the scoped Snow Boots placement deferral, the Overwield pre-break
candidate and its negative-state fix, and the Creative armor rebuild. It keeps
the #323 splashing ability accounting and bottle changes with the later
Gilded/Speedy/Step Up and ordered stat paths. The older #318 post-removal spend
is superseded. The exact merged head requires its own package, official apply,
type and game gates; #323's package and planner checks do not establish those
results for the merge.

## Melter pointer and package namespaces follow-up

This follow-up carries #337's retained remote screen position into the Melter gauge transfer entry point. The b53 UI diagnostic covered can transfers, fuel and incompatible fluids; exact integrated-head apply/build and live behavior remain pending. The content package generator declares `mantle` and `forge` as additional namespaces because its normal package actually contains `assets/mantle/lang/en_us.json` and `data/forge` entries. The declared namespaces are subject to the target host's normal collision and content checks; package generation alone is not a resource-preparation pass.

## Brewing selector follow-up

The exact #333 official apply rejected `horizon_client::stations::StationData::load` as an unresolved target. A separate diagnostic reached the owning `horizon_client_lib` target but rejected its `Self` return. This candidate instead selects the concrete `crate::brewing::Brewing::load(root, items, overlay)` expression inside `horizon_client_lib::stations::StationData::load` and adds the same bottle conversions to its owned `Brewing` value. The expression and item argument are from the target host, and the existing conflict/idempotence guards remain. This new selector is proposed pending official plan/apply/type and exact-host brewing tests; no selector or safety check is bypassed.

The exact #342 combined apply reached `source-empty-bottle-use` and reported `E_INVALID_BINDING`: the helper's `&ItemStack` spelling differed from its manifest's `&horizon_items::items::ItemStack`. The follow-up uses the manifest's explicit type spelling in the helper. This resolves that reported mismatch only; the full subsequent apply, type/build and gameplay still require validation.

## Molding fallback callback

In upstream `CastingBlockEntity.interact`, the first matching molding recipe calls `result.onCraftedBy` once. If a held item did not match and the subsequent mold-less fallback succeeds, the source instead clears the input and gives `recipe.assemble(...)` directly, with no `onCraftedBy`. The normal-resource #93 fixture's blank-cast crafted statistic after an unrelated plank interaction therefore exposed an extra notification, rather than an expected source statistic. This candidate removes only the fallback callback/stat increment; a direct matched molding recipe, including an empty-hand match, still takes the original callback path. Recheck unrelated held items, direct molding, empty hand, occupied output and failed storage, with item counts and callback/stat observations. A mod callback observer and exact-host gameplay remain unverified.
