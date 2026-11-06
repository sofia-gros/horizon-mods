# Latest language overlay repair: source design gates

Unimplemented design against official Horizon
`f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`. This updates the assumptions in
[the historical 624c proposal](language-overlay-compatibility.md); it is not a
runtime patch, accepted Mixins placement, test result or new locale owner.

The latest Pack8 fixture and its reviewed controller remain frozen at
`15397f42` and `dfd09ee7`. They characterize existing native behavior, not this
repair. No trigger is created by this document.

## Keep the two pipeline owners distinct

Latest native `packs::merge` now calls public `lang::read_lang_file` when a
language layer is not a strict JSON object. A commented pack can therefore
survive as native-prepared string values. Selected/server order and filters are
still processed before `mod_content::build`. Its ordinary `Overlay::write`
still publishes the admitted Mod file as a whole.

The proposed bounded compatibility change remains **surviving lower-only key
retention with existing Mod-wins conflicts**, not packs overriding Mod defaults.
The only lower input is the already prepared, filtered base path. No raw pack
resurrection, reverse merge, `Language::load(...).map()` extraction, independent
CURRENT, namespace admission change or retained-UI authority is permitted.

The existing two-strict-flat-string domain has a new consequence: native
lenient pack output may now qualify because the official pack owner already
normalized it. A raw lenient Mod file still does not qualify; its original bytes
must follow native whole-file behavior. Strict pack numeric/boolean values are
not normalized by that merge branch and remain outside this candidate domain.
This is deliberately incomplete key recovery, not support for every legacy
language file. Empty-object retention is still an intentional behavior change.

## Public reader is not an eligibility oracle

`read_lang_file(bytes, &mut map)` returns no acceptance result. It can preserve
an existing map on invalid input, or add only an initial part before a null,
array or object entry ends its lenient loop. A nonempty or unchanged resulting
map therefore cannot prove that a complete strict flat-string object was read.

A future implementation must separately establish the exact bounded strict
object/string domain using the existing native serde dependency, or obtain a
proper native parse contract. It cannot treat the now-public reader as a
success/failure validator. Any transient preparation maps must remain local
values, not a second lookup dictionary. Private renderer merger exposure and
small writer placement still need an actual latest plan/borrow/type proof.

## Policy identity must enter the native hash, not its suffix

Latest `resource_root` chooses `cache/mod-content/<stamp first 16 characters>`.
The private `stamp` hashes length-prefixed inputs, beginning with the native
domain string; executable metadata is only one input. Appending a policy name
to the returned hexadecimal stamp would leave its first 16 characters unchanged.
It can invalidate the stored full stamp while rebuilding the same directory,
which does not provide the new-root separation needed for retained old owners.

The candidate must instead put a fixed policy/version discriminator into the
native stamp's hashed input sequence. Keep native metadata ordering and error
behavior, hexadecimal output shape and all original inputs. This is a proposed
additional exact-source seam, not an available public hook. An actual plan must
prove an admissible exclusive transformation; no large resource_root overwrite
or unproven RETURN hook is assumed. Native truncated-hash collision semantics
are not strengthened into a general uniqueness guarantee.

Latest cache hits refresh the stamp file, and builds can prune stale roots.
Do not manually delete caches or bypass pruning to obtain passing evidence.
A bounded future fixture must own all roots and prove its fixed old/new policy
inputs choose different paths; the old retained root stays readable, a native
same-policy reuse stays valid, and write/read failures keep native phase and
caller fallback behavior. Pruning/age cases need explicit fixture-owned roots,
not user cache directories. The current Pack8 calls `build`, so it proves none
of these cache operations even if all its cases pass.

## Minimal acceptance order

1. Run latest Pack8 unchanged: strict/lenient cases, selected/server/filter
   precedence and Mod whole-file replacement. Record baseline success/failure.
2. Prove latest module placement and exclusive small writer/stamp composition
   before implementing activation. Preserve admitted package read errors,
   namespace-clash timing, directory materialization/link detach/write order.
3. Compare only intended key-retention differences in the eligible domain.
   Unsupported/over-budget/partial/lenient Mod inputs keep exact higher bytes;
   filtered paths stay absent and placeholders are not rewritten.
4. Separately exercise native resource_root policy transition, cache reuse and
   owned failure/pruning cases. Neither equal locale code nor a loaded Language
   value supplies installation authority.
5. Only then test the existing Game applied/live Mod and locale-reload owner.
   Key/Text spans remain snapshots; Game/Options, retained Guide/LAN and real UI
   refresh are separate proofs, not implied by preparation or cache success.

Exact source evidence is in
[the 33-file latest audit](latest-language-fluid-delta-20261106.json):
`mod_content.rs` SHA256
`a8c6cea6ee73824d704699d0d68775cbdb60d19c51a7f406336874f3350e1e6c`,
`lang.rs` SHA256
`743734901c1fd5e0c3c2822ea0207d46c4ac04ad7768b6d93f3b0e235e72f417`,
and renderer `packs.rs` SHA256
`e13ff1e4a7fb69f2e7a4bae07e8c56bf48d53d0dea58d353c4ac6b238f95e783`.
This document performs no native execution, runtime/schema change, official
repository edit, production activation or upstream communication.
