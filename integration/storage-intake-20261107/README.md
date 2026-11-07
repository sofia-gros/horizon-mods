# Storage/machine PR intake: first implementation group

Base: `9e032ef5b013487ea08b997eb08df84825fb09ba`. This candidate does not update main or the integration branch. Issue #379 was read through the normal API; its latest recorded update is 2026-11-05T14:41:42Z. Its priority remains areas 2/3/4 high, then medium, followed by area 1 high/medium and finally low.

The 70 `storage_machines` PRs are grouped without declaring unreviewed features absent or complete:

- Portable casting/drop preservation: 2.
- Machine save/inventory transactions: 22.
- Fluid route staging: 31.
- Fuel quantity/acquisition: 5.
- Signals/comparators: 3.
- Duct filters: 2.
- Casting input/repair policy: 5.

`tracker.json` records every PR and its frozen head. Grouping is not individual semantic reconciliation. Only #918 and #922 have the focused result below; the other 68 retain their unfinished status. Historical whole-file differences often include later shared migrations and do not establish missing behavior.

## Existing behavior and the new delta

#918 is already included: placement and removal sources match its production files, and `copy_native_tank` retains its exact raw-transport body. It is not reimplemented. The three outer-tank delegations already integrated from 794a are also unchanged.

#922 is already included for native portable tank admission, unknown tank/custom-data preservation, exact-current-tank checks and tagged/untagged filling. The original portable functions remain byte-identical before this candidate. The recipe routing is present. This is not a claim that the whole PR was missing or that its gameplay/save/LAN behavior has now been validated.

Two residual local codec predicates in `portable_tank` and `fill_portable` are replaced with the existing `casting_item_save_safe`. That helper already delegates to `pluto_stack_storage::encode_item_lossless`. No new public API, registry, hook, module or package entry is added.

The consumer keeps capacity, item count/damage, aliases, native tank schema, exact fluid/tag equality, quantity, recipe ordering, raw metadata patches, and publication responsibilities. Pluto supplies recursive retained-Wire refusal and the native codec roundtrip comparison. The input and staged result both use that same boundary.

On inputs admitted by the existing root-Wire and count=1/damage=0 checks, the common function uses the same count/damage/component comparison as the previous local code. It additionally refuses retained Wire in nested container, charged, pot-decoration and bundle stacks, including stale backing. Native component equality omits that backing, so a successful old roundtrip alone did not establish preservation. This added refusal is deliberate and is not a claim to transport, erase, normalize or recover opaque data.

## Validation and remaining work

Eight source/inverse Python checks pass. Five actual-type Rust regression tests were added but have **not run**: root Wire, all four nested fields with live/stale backing, deeper recursion, positive plain nested/unknown UTF-16 preservation, and the existing tag/capacity policy. The old local acceptance predicate is exercised as a negative-control expectation in the nested test, pending native execution.

`inverse.json` proves the only production changes are the two predicates plus the test append, and restores the exact prior source. Existing helper/manifest changes, #918, the outer delegations and stage/launcher/CI are not modified. The prior 153/16, outer160, Guide10 or game compile results do not validate this new candidate.

No Cargo, metadata, Rust compiler, new CI or local game execution was performed. Independent review and a later authorized native gate are required before promoting the candidate. Host authority, save/reload, notifications and LAN remain separate verification boundaries.
