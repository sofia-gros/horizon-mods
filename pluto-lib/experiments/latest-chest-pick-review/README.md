# Tinkers chest pick-color boundary: independent source comparison

This is a read-only audit of the next full646 diagnostic, not a new runtime adapter, accepted plan, native type check or game test. Main remains `08afa992d5091e03360f3473ea93fd17aa793987`; latest official remains Horizon `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd` and Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`. Exact source hashes and proposed differential cases are in [evidence.json](evidence.json).

## Observed failure and unchanged native body

[Full646 run 42215886835](https://github.com/sofia-gros/horizon-mods/actions/runs/42215886835), exact `74fc9a53e4bde161f326bbd16710b625f86716e6`, reached a terminal plan rejection after 580.106 seconds:

- `E_UNSUPPORTED_CONTROL_FLOW`, `tconstruct_native / tinkers-chest-pick-color`.
- Target `horizon_client::game::block_items::Game<'_>::picked_stack`.
- Message: `closures, async, try/?, unsafe and const blocks are unsupported injection contexts`.
- The original 644-to-646 inverse, four changed paths and six named preserved mining-policy files passed. No accepted full plan or collector result was produced; no apply/type/actor proof follows.

The exact f4dc `game/block_items.rs` is 17,218 bytes, SHA256 `0bbdbf91760376e6e97b9769048e7f827f1a3e16c478872408429d5e389c8dd7`, byte-identical to the previous official 624c export. Do not attribute this failure to a new native method body or infer that an earlier full TConstruct RETURN hook passed. The rejection identifies an unsupported transform context, not the precise offending AST expression.

## Existing consumer policy, including edge cases

Main's RETURN helper owns an `Option<ItemStack>` result. `None` and every item except exact `tconstruct:tinkers_chest` return unchanged. For that item it reads saved metadata at the formal block position. Only `Tag::Int(color)` sets `stack.data_mut().dyed_color = Some(color)`; missing or wrong-typed color leaves even an existing color unchanged. The old helper does not clamp the signed integer, require creative mode, inspect Ctrl/include_data, add a custom-data field, rebuild the stack or commit to inventory.

The metadata helper is part of the policy, not interchangeable with a generic nested-key lookup:

1. If the raw source contains `tconstruct:modifier_chest`, return that nested value first, even if it is malformed or the raw ID differs.
2. Only if that nested key is absent, and the raw `id` resolves to `tconstruct:modifier_chest`, construct the legacy view from top-level `color`.
3. First matching source record at the position wins. A malformed nested value does not fall back to top-level color.

Placement stores the stack's modeled dyed color in chest metadata; removal/drop uses a separate pending copy and restores color. Those placement, drop, tint and metadata policies are not being moved or changed by this audit.

The safe migration target preserves the **actual native return value** and edits only its modeled color after native collection. Encoding and decoding semantic components to reconstruct the stack would introduce a new loss boundary. Preserving a retained Wire field is not a guarantee that it remains usable after a modeled component changes; arbitrary wire/save roundtrip or transport correctness is not claimed here.

## Native phase and authority

Native `picked_stack` chooses the clone item, rejects air through `?`, creates a count-one stack, collects the applicable block-state/banner/pot/entity patch, decodes that native patch and finally returns `Some(stack)`. Chest color is a post-collection amendment, not permission to skip native item selection or copy arbitrary block NBT.

Three observed exact-source callers are recorded, without claiming a fresh complete repository call graph:

- Local middle-click calls `picked_stack` for either survival or creative. `include_data` is `creative && Ctrl`. Remote mouse handling delegates before this local branch.
- LAN server handling checks loaded state, eye/range and uses `as_guest`, then calls it with `creative && packet.include_data`. The later guest pick callback may cancel; changed-slot/held-slot synchronization remains native.
- The creative hive-drop path also calls it under its native hive/block-drops gates. Unrelated output items must remain unchanged.

The real `pick_item::pick` selects by `same_item_same_components`. Survival may select/swap a matching colored stack but cannot create one; creative may clone the prepared stack if no match exists. Consequently, adding a new creative-only gate would regress ordinary colored-stack selection.

The latest `native_player_pick_item` callback is not an equivalent replacement: the observed path is guest-side, it takes no result stack, and its explicit output is cancellation. Local middle-click and the hive path do not pass through that callback. Reconstructing or committing inventory there would move authority and ordering.

## Existing PlutoLib reuse and missing boundary

The actual common archive `4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d` was inspected, not just an uninstalled source directory. Its Rust payloads contain no `picked_stack` or `native_player_pick_item` attachment.

- `ItemFacts`/component views are semantic reads, not replacement or commit tokens.
- Stack canonical/lossless codecs and snapshots do not provide a native pick-result interception point; a codec roundtrip is unnecessary here.
- `patch_compound` can preserve caller-owned NBT fields but does not read world metadata, select a native return value or authorize mutation.
- ItemBehavior/ItemData covers use events and the owned integer compound `plutolib:data`, not the modeled `dyed_color` or pick results.
- Presentation/render helpers do not change inventory component identity.

The missing reusable boundary is a synchronous, officially accepted way to amend the already-created native pick result while retaining its formal position and native success/None decision. Its validity still needs an exact official plan and type proof. There is no reason to introduce another global map, TLS, color registry, codec reconstruction or inventory writer. Storage owns investigation/implementation of that boundary; this audit does not select or enable an unproven hook.

## Next evidence

Use the cases in `evidence.json` to compare the unchanged consumer policy against any proposed adapter. Separate native body preservation and complete composition from tests of the tiny color operation. Test metadata precedence, unmodified unrelated fields, both include_data values and native survival/creative selection. Preserve the original failure receipt and mining36 success; neither proves chest adapter or complete Game behavior.

No Rust, Cargo metadata, new plan, apply, trigger, production manifest or repository-state change was performed for this audit. The only publication is this documentation and its source/evidence ledger.

## Subsequent source check: one final owned LOAD candidate

The exact latest Mixins `plan.rs` and `scopes.rs` clarify the rejection. RETURN rejects an outer `?` because it cannot wrap that early exit; ordinary synchronous closures elsewhere in this body are not by themselves rejected. MODIFY_VARIABLE explicitly allows `?` outside its selected value and leaves that control flow unchanged.

The static visit order of local `stack:0` gives four LOAD matches: `components(&stack)`, the `stack.damage` assignment place, the `stack.extra` assignment place, then the owned `stack` in the final `Some(stack)`. References and fields are counted but marked unsupported places. The field visitor resets `ignore_reads` to false even on this assignment LHS. Only the fourth is a non-place candidate. Thus require=4 / ordinal=3 is a source-derived hypothesis to verify in a small exact official plan, **not an accepted ordinal or authorization to install it**.

Amending only that final owned value would leave the native `? -> None` path alone. The old helper does nothing at all for None, so not invoking it on that path is semantically consistent. On the Some path, all native patch collection and decode already happened. This supports preserving the original color policy without copying the entire native body, recursing into it, adding TLS or inventing a creative gate. Exact tail/function/variable fingerprints, actual selected IR, helper typing and complete composition are still required before relying on this seam. The old policy and all ordinary manifests remain unchanged.
