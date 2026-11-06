# Owned chest-pick result candidate

Source-only, unapplied, and not a playable or type-checked build. This first checkpoint
remains pinned to Horizon `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd` and Mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57`. Subsequent official updates require a
separate cutoff. Original main, archives, mining candidate and earlier proofs are unchanged.

## Actual obstruction

Full646 source `74fc9a53` ran once as run `42215886835`, job `127885664173`.
The planner returned `E_UNSUPPORTED_CONTROL_FLOW` at `tinkers-chest-pick-color`,
target `Game::picked_stack`. Closure `5db8371b` preserves the exact result.
The file is byte-identical in old624c and f4dc. This is a pre-existing unsupported
RETURN target exposed when planning reached it, not a latest-source regression.

Official `plan::validate_target` rejects the outer `picked_item(...)?` for RETURN.
Its MODIFY_VARIABLE mode explicitly preserves `?` outside the selected value.
Official `Scopes` counts reference and field-place reads but refuses to rewrite them.
The four `stack` LOAD candidates, in source order, are `components(&stack)`,
`stack.damage`, `stack.extra`, and the final `Some(stack)`. Only the last is an owned
non-place read. Thus require4/ordinal3 is a source-derived proposal, not a proven IR
coordinate. An exact-source small official plan must verify the target, local,
LOAD ordinal/count, selected expression, output/value contract and bindings before
this candidate is used in another full plan. Rust type checking is a separate gate.

## Exactly two candidate payload changes

1. `tconstruct_native/horizon-mixin.toml`: only the existing chest injection changes
   RETURN to MODIFY_VARIABLE, adds the four-read/last-read selector and changes its
   `$value` binding from `Option<ItemStack>` to `ItemStack`. Same owner, priority,
   target, helper filename, Game reference and position. No new hook is added.
2. `tconstruct_native/source-chest-pick.rs`: exactly two lines change. Its input/output
   becomes the owned stack and the old `Some` extraction becomes a mutable borrow of
   that value. Everything from the item guard through the exact metadata read,
   Tag::Int test, dyed_color assignment and final return is byte-identical.

This is a narrow TConstruct compatibility bridge using existing official Mixins
capabilities. It is not a new Pluto API or an additional Pluto migration claim.
Existing Pluto APIs do not provide the same position-aware picked-result phase.
No new registry, callback bus, TLS/reentry, whole-body replacement, stack clone,
codec round-trip, inventory mutation, save or notification operation is introduced.

`candidate.transform_payload` only accepts the sealed full646 mining candidate
`02882fbd...ceb823`, changes those two paths, and verifies a complete inverse to the
same646 payload. Mining b78, common source, all other payloads and owners remain
unchanged. This whole-payload gate is prepared but has not run on a local recreation
of all646 files; the earlier full proof supplies the expected seal. The two-file
inverse and exact original manifest/helper checks have run as pure source tests.

## Policy and ordering

The native function finishes its existing component patch/decode before the selected
owned value is edited. `None` from the native `?` never reaches the new hook; the old
Option helper did nothing for None. Only a `tconstruct:tinkers_chest` with an Int color
returned by the same storage read is changed. All signed i32 values are retained,
including negative values. Missing/wrong-type color leaves existing dyed_color alone.
The storage reader's nested/legacy precedence is not reimplemented here. It remains
the same exact call. No creative or include_data condition is added: survival uses
the colored stack for inventory matching too. Other fields and retained/unknown
data remain on the original owned stack.

The guest-only `native_player_pick_item` cancellation hook is not an equivalent
replacement: it has no picked-result stack and does not cover local picking.
Independent policy/Scopes review is preserved at `ed6c9e1e`.

## Verification and exclusions

16 Python source/negative tests pass, including exact two-line helper difference,
manifest semantic delta, original helper/manifest inverse, mining hook preservation,
and fail-closed input drift. Native IR, Rust helper type, Game actor paths, complete
plan, old12 guards, apply/build and gameplay remain unrun for this candidate.
Actual ItemStack/Tag differential fixtures are owned by the separate recipe reviewer.

Bounded source preflight in `preflight.json` covers the remaining15 main non-HEAD
hooks/10 targets after the chest hook. It found no additional obvious `?`, async,
unsafe, const, yield or try token candidates after stripping comments/strings and
manually reading the selected bodies. This is not a Rust AST or macro/selector/type
proof, nor a prediction that those hooks will pass. Other failure classes remain open.

Read-only follow-up at official `41f822b659789780f58d32c63ddfb6887ea1b8bf` found the
whole block_items.rs still identical, and native_break_speed/native_player_pick_item
function bodies identical. The native_hooks.rs whole-file hash changed. No current
latest compatibility or source-pin equivalence is inferred from those limited facts.
