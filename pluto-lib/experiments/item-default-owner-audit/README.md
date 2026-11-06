# Latest Items-owned default-name boundary: read-only design audit

Target: official Horizon `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd` and
Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`. This is an unapplied
source-level proposal. No state, hook, module, public API, manifest or CI input is
changed. There has been no planner, metadata, compiler or runtime execution for
this owner proposal. The frozen Template6/default4 CI and old successful
624c-only item-name32 proof remain separate.

## Constructor census and its exact evidence boundary

The immutable official recursive Git tree returned 5,956 entries, not truncated.
Its only path at `crates/horizon-items/src/items*` is `items.rs`; the complete
owner module was read and its Git blob identity verified. `lib.rs` directly
exports `pub mod items` and forbids unsafe code in this crate. The owner has one
`impl Items`, no Clone/Default/serde/layout derive or owner alias, and private
`mob_visibility` and `loot` fields. Other modules/crates cannot use ordinary
struct literals or struct-update syntax to construct that type through those
private fields. The owner's child test modules are inline in the same file;
there is no external child module, `#[path]`, `include!`, custom macro definition
or Items alias hiding another constructor there.

There are three structurally visible owner literals, all spelled `Self`:

| Function | Current source location | Configuration | Required outcome |
| --- | --- | --- | --- |
| `Items::load_with` | lines 745–994; literal 980 | unconditional | Initialize empty owner state, then populate from the same live `raw` on success. |
| `Items::tags_only` | lines 1029–1043; literal 1030 | unconditional | Empty owner state, no prototype population or extra read. |
| `Items::for_tests` | lines 1046–1064; literal 1047 | `cfg(any(test, feature = "test-support"))` | Empty owner state in both relevant configurations. |

`Items::load` and `Items::catalog` delegate to `load_with` and do not construct
another literal. The `test-support` feature is declared by the actual items
manifest and used by downstream tests. Existing `Self` constructors for unrelated
types in this file must not be counted as Items constructors.

Repository-wide indexed searches for the private member and the explicit test
factory were also checked: five private-member paths and nine test-factory paths
were returned at the same immutable commit. They are supporting discovery only,
not a substitute for a whole-repository parser or compilation. This census is
privacy-bounded: it does not claim every repository file's bytes were lexically
searched, nor rule out arbitrary unsafe fabrication outside this crate. A later
actual planner must independently enumerate all visible constructors, and a
check must cover normal, test-support and test configuration before state is
adopted. In particular, `for_tests` configuration resolution and the large
`load_with` body's transformability are still unproved.

## Preferred candidate: state plus successful-tail population

1. Add one private owned state to `items::Items`, with a standard empty
   initializer. The payload is a map of copied raw default-name values keyed by
   existing item IDs. It must not retain the native `Arc<Defaults>`, resolved Text,
   a global map, root-key cache, TLS or pointer identity. Use the existing items
   crate's serde_json/world dependencies, not a new consumer-to-host dependency.
2. Declare each of the three exact `Self` constructor sites, one per function.
   Default-empty state is correct for tags-only and test tables. Do not silently
   omit a conditional constructor to make a plan pass.
3. Prefer an `AFTER_CALL` candidate on the one tail **call** `Ok(Self { ... })`
   inside `Items::load_with`, selecting `Ok` with `require = 1`, `ordinal = 0`.
   The earlier `Ok(bytes)` is a match pattern, not a call. Bind the owned result
   mutably through `$result_mut`, and borrow the existing local through
   `$local_ref:raw:0`. The proposed concrete types are
   `&mut crate::Result<crate::items::Items>` and
   `&std::sync::Arc<horizon_world::item_defaults::Defaults>`.
4. On the successful result, copy only the selected raw item-name payloads into
   that result's state. `raw` is already the snapshot used for the native item
   table. Do not reread files or call the defaults hook again. Release the raw
   Arc at the existing function exit, leaving only owned payload values in Items.

This is narrower than copying `load_with` or adding a RETURN hook that would
need `raw` at early `?` exits. It also avoids putting constructor-local capture
inside a state initializer, which official Mixins explicitly forbids. The
alternative owned-literal expression modifier is documented as supported in
principle, but requires an exact full literal selector; it is not an additional
planned hook. Neither alternative is claimed accepted here.

The latest docs specify ordinary parameters and lexical captures for AFTER_CALL,
one evaluation of the original operation, hook execution only after successful
call completion, and owned result forwarding without Clone. State initialization
occurs after original fields, independently of constructor-local captures. The
original field expressions include fallible calls: an error or panic before the
tail `Ok` must not populate a partial owner. Actual plan/apply/type checks must
prove this particular nesting/capture/state composition, not just cite the docs.
Normal Rust allocation failure is not claimed recoverable or transactional.

Composition also needs an exact source/site guard. Another hook can change the
raw snapshot, replace the returned table or bypass the population phase. Do not
silently assume a producer runs or forbid unrelated mods more broadly than a
reviewed complete-plan contract requires. No guard relaxation is proposed here.

## Display is a different boundary

Existing native `styled_name` resolves custom_name, then explicitly stored
item_name, then a String fallback. It has no Items/default snapshot argument.
The Game hover fallback separately handles a nonempty written-book title,
explicit item_name, ominous banner, potion, player head and the standard item key.
The held-name HUD invokes styled_name with a different `name_line` fallback.

Therefore a generic prototype-name shortcut at the front of styled_name can
incorrectly override written titles or modeled/special native names. Returning a
String also loses styled Text. The minimum first owner capability should be a
read-only borrowed payload lookup, not a second display resolver registry.
Production display call sites must later use the actual Items snapshot, preserve
all existing priority branches and return the existing native Text type. An
explicit removal suppresses prototype item_name, not an independent custom_name
or standard fallback. Explicit per-stack names remain distinct from defaults.
Raw translation keys/arguments/styles must resolve under the current language,
without mutating ItemStack, saved components, Wire, count or network identity.

The old isolated removal32 success does not prove this default owner, latest
packet semantics or coexistence with the common package. Supporting named
components in one UI path must not be described as covering every Game display.

## Required later proof, not execution permission

- Exact owner/constructor plan under latest official sources, including cfg paths.
- Actual owner field initialization and successful-tail population apply/type proof.
- Normal/catalog/tags-only/test construction, error before/inside the literal,
  repeated same-root load, two roots and object destruction; no retained raw Arc.
- Translation key/args/style preserved unresolved; local read never changes save
  or raw transport state; explicit removal/custom-name priority remains distinct.
- Actual display call-site order and Text preservation, with special names intact.
- Full relevant composition guards and unchanged native fallback without a payload.

Template6/default4's future results are limited finite native contracts. They do
not satisfy these owner/population/display gates by themselves.
