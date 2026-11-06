# Excluded language-policy root-key proposal

Source only, not installed or executed. This is a **different cache contract**
from adding policy to native `stamp` inputs. Ordinary Pluto/TConstruct runtime,
the successful Pack8, helper14 source, rejected placement `150acf22` and its
failure evidence remain unchanged. This proposal does not merge any language
file and is not a working repair.

## Narrow caller seam

Exact official Horizon
[f4dc resource_root](https://github.com/horizon-rust/horizon/blob/f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd/crates/horizon-client/src/mod_content.rs#L2714-L2743)
first returns the unchanged base for no content, computes the native stamp,
selects `cache/mod-content`, and initializes `out` with the first 16 stamp hex
characters. The proposed single MODIFY_VARIABLE STORE changes that owned `out`
value only. Its other binding borrows the complete native `stamp` already in
scope. The actual planner must still accept the selected local, extra binding,
helper placement and types. No planner/apply/type check has run.

`root-store.rs` hashes these ordered UTF-8 parts with SHA256, each preceded by
its byte length as little-endian u64, matching the native encoding style:

1. A fixed root-policy domain.
2. The exact existing merge helper's policy ID, source-pinned here.
3. The complete 64-hex native stamp, not its first 16 characters.

It replaces only the basename with the first `ROOT_NAME` (16) hex characters.
The parent roots directory and native pruning name shape remain unchanged.
No suffix, partial native hash, or policy-only common prefix is used as the
folder key. The probe embeds the existing policy string because it deliberately
has no merge module or writer hook; its source guard requires byte agreement
with the unchanged helper. This is not a second policy registry. A future policy
meaning change requires updating that existing ID and this excluded contract
before it could be composed.

## What stays native

The in-memory composition changes one initializer and appends the exact helper.
Removing that delta reconstructs the entire original mod_content file. Native
stamp computation and failures occur first. Native stamp file bytes and exact
comparison stay unchanged. Warning reads, touch-on-hit, build-on-miss, stamp
write, prune, and return/error order remain as written. Existing roots are not
renamed or deleted by the helper. Policy presence can cause a cache miss even
when a particular content set has no eligible locale files. Filesystem and
allocation failures after the changed root selection remain separate from this
pure source comparison.

A root key is **not** a stronger content hash: native stamp still depends on its
original metadata/input scheme. Changing this helper does not fix same-size,
same-mtime inputs that native stamp does not distinguish.

## Collision limitation, not a hidden safety promise

Length prefixes prevent structural ambiguity between the input parts and the
full native stamp avoids trivial shared-prefix keys. SHA256 is still truncated
to the native 16-hex folder shape. Therefore this proposal does not prove a
collision-free namespace. In particular, two policies with the same native
stamp could theoretically select the same folder. Because its stamp file still
contains only the native stamp, that collision would not detect the policy
change. This differs from putting a policy-dependent full stamp into the file.
It is an explicit unresolved cache-identity limitation, not repair completion.
A policy-aware persisted identity would require another independently justified
boundary. No new stamp file, side registry, cache bypass or native body copy is
introduced to hide it.

## Alternatives excluded by exact Mixins rules

The executed `150acf22` probe rejected direct `stamp` STORE because the function
contains a local `use` item. Exact Mixins
[5a64 validate_target](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/plan.rs#L919-L980)
and [visit_item](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/plan.rs#L1177-L1186)
also exclude RETURN on that same target. Moving to the caller's `stamp` local
would select `stamp(base, mods)?`: its Try expression is rejected by
[modified_value_error](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/plan.rs#L1036-L1051).
The allowance for `?` outside a MODIFY_VARIABLE's selected value does not waive
this selected-value rule. These alternative limitations are source findings,
not newly executed diagnostics. The exact plan.rs SHA256 is
`5226f837305a778f73cf428411d5cf7a1cf1b8b33de2fd603a1459e7863e32db`.

## Current validation and future gates

Nine Python checks cover fixed actual source identity, whole-file inverse,
original call order, exact one-hook/binding declaration, policy agreement,
length-prefixed hash-contract examples, and input/path rejection. They do not
execute the Rust helper. Source inputs are assumed quiescent; checks do not
claim protection against a concurrent symlink swap.

Future gates remain distinct: exact tiny plan acceptance; actual locked sha2
helper types/tests; applied native writer/build/hit/miss/prune/error fixtures;
complete installed composition and guards; finally Game/UI/reload behavior.
The 16-hex policy-collision limitation also needs a conscious contract decision
or a separately proven persisted-identity design. No execution or activation
is authorized by this source checkpoint alone.
