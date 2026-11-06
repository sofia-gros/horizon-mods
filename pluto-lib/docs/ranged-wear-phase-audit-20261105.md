# Active ranged-wear invocation boundary

Read-only source investigation at MOD `4d82580b32a1c040001add8c853d93d331bc493a`, official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`, official Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`. One read-only official plan probe was attempted and rejected, as recorded below. No apply, native compilation/test, mutation API or consumer migration was performed. Manifest-active does not mean full TConstruct composition is verified.

## Exact consumer and native order

The active `ranged-retained-wear` injection is a cancellable HEAD on `horizon_client::game::Game<'_>::damage_held` (`tconstruct/mixins/tconstruct/horizon-mixin.toml:2057-2066`). Its helper `ranged-wear.rs` does:

1. Clone the current acting hand.
2. Continue into native code unless this is a material-bearing TConstruct longbow/crossbow (`source-ranged-wear.rs::is_material_bow`).
3. For a noncreative owned bow, call `source-overslime-host::damage` once, receiving loss and overslime spend.
4. Re-read the current acting hand, compare item ID only, spend TConstruct overslime and apply TConstruct retained-tool damage.
5. Break, bypassing the entire original native `damage_held`, including its native `wear` call and broken-stack tail. Creative owned bows also Break without mutation.

The current custom calculation calls `durability_loss`, **not** native `wear`. It temporarily adjusts the per-actor modifier-cause context and restores it after ordinary return. Native `durability_loss` can consume enchantment RNG; it does not itself call `native_item_damaged`. Adding the full native wear sequence during extraction would change current consumer behavior and requires an explicit recovery decision.

Official `Game::damage_held` (`game.rs:13603-13636`) instead:

1. Returns immediately for creative.
2. Clones the current acting-hand stack and invokes `wear` if present.
3. Re-reads the acting hand to choose max durability, including trident handling.
4. Borrows the current acting-hand cell, applies loss, removes a broken item, awards a broken statistic and emits native equipment-break feedback.

Native `wear` (`game/enchant_effects.rs:90-108`) executes durability RNG/policy, a server player's `native_item_damaged` hook with cancellation/point adjustment, then item-durability advancement when loss is positive. The advancement trigger is at `game/advancements.rs:1361-1387`. These effects precede the slot mutation and cannot be rolled back by rejecting a later write. Any shared service must distinguish calculation, effect execution and commit, and must not rerun the first two.

## Capture each wear invocation, not the whole shot

TConstruct longbow release (`material-bow-release.rs`) delegates to native `release_bow`. The latter (`game.rs:14269-14360`) resolves/looses ammo, constructs and spawns a projectile, invokes shoot callbacks and projectile advancement, consumes sound RNG, plays audio, awards usage, consumes ammo, and only then calls `damage_held(1)`. TConstruct's active projectile-add helper can already apply Glowing wear while the projectile is added. A shot-entry equipment snapshot is therefore not the correct expected value at this later wear invocation.

TConstruct crossbow use (`material-crossbow-use.rs`) delegates to `shoot_crossbow`. Native `game/crossbow.rs:290-408` first takes the charged-projectile component out of the live bow and tidies it, then creates each projectile, invokes shoot callbacks/advancement/audio, and calls `damage_held` once per projectile (3 for a rocket, 1 otherwise). Shot-crossbow advancement and used stats follow the loop. A shared service must preserve these multiple invocations and must not compare the post-clear bow against its pre-clear component snapshot.

Current longbow/crossbow helper tests use actual consumer helpers and item types but explicit Game/dispatch doubles (`tconstruct/tests/rust/longbow_release_threshold.rs`, `crossbow_primary.rs`). They are not evidence of native wear/actor/slot authority or single execution of the real native tail.

## Physical hand and actor scope

`held_slot` (`game.rs:3841-3846`) resolves Main to the current `selected` index and Off to `shield::OFFHAND_SLOT`. `held` and `held_mut` re-evaluate that function (`:3718-3723`); they do not retain an earlier physical slot.

Local item use enters `with_hand(used_hand, tick_used_item)` (`:7258-7276`). `with_hand` saves/restores the hand enum around the callback (`:3858-3863`), not a frozen selected-slot or transaction token. LAN release runs `as_guest` and `with_hand` before `release_used_item` (`game/lan_actions.rs:2120-2143`). `as_guest_with_host` temporarily swaps guest state and restores it after the callback (`:340-354`). These native scopes explain the intended active actor but are not a public immutable lifetime/generation capability.

A new service could capture the actual physical index, original native value, hand and observed actor/world facts at **damage_held invocation entry**. Requiring these to remain unchanged would be a new fail-closed policy, not a purely mechanical extraction of today's late-bound writes. Current read-only actor tuples cannot distinguish replacement by a new world/actor with the same tuple. The guide worker's client `Remote::native_epoch` is not authoritative equipment identity and even survives same-key respawn.

## Supported hook and borrow seams: evidence limits

- A plain HEAD and a support module are documented for lifetime-bearing Game methods. The current consumer already declares this shape. A HEAD alone cannot carry a local snapshot through an otherwise unchanged function body without another owned scope mechanism.
- `WRAP_OPERATION` can forward a target reference-parameter receiver, original arguments and an at-most-once continuation (`docs/advanced-injections.md`, core `plan.rs:1236-1303`). `self.wear(&held, amount)` uses a simple target receiver and does not mention self in its arguments, so it meets those **source-level receiver conditions**. The actual plan probe below was rejected before receiver forwarding or Rust borrowing could be established; no earlier worker result proves either.
- A wear wrapper alone cannot veto the enclosing commit by returning zero: `damage_held` still re-resolves the current slot/max and may remove a stack whose damage is already at its maximum even with a zero increment. It also cannot undo RNG/hooks/advancement.
- The official planner's `Sites` visitor (`sites.rs:126-181`) marks whole-expression calls at statement/plain-let/block-tail roots. The selected wear call is directly inside a match arm of `let amount = match ...`, not one of those statement roots. Core `plan.rs:602-606` rejects BEFORE_CALL or cancellable AFTER_CALL at such a non-whole-expression site. This is a source-derived restriction, not a recorded tool denial.
- Pattern-bound `held` is not an eligible lexical local capture (`docs/scopes-and-bridges.md:23-34`). AFTER_ASSIGN requires a whole call initializer; this initializer is a match, so it does not provide a post-calculation captured-stack bridge either. Ordinary RETURN is too late to guard writes.
- A lowest-priority cancellable HEAD with a reviewed exact-body adapter could establish a private stack-frame scope, as the existing armor observation adapter does. But it would require a complete source/composition guard rejecting bypassed interior hooks and proving RETURN handling. It is not currently implemented or approved as a mutation path, and any newly frozen-slot rejection remains an explicit behavior change.

## Borrowed-capability conclusion

A borrowed slot capability can safely prevent same-frame reentrant mutation only during a callback that receives that slot and **cannot simultaneously borrow the containing Game mutably**. The existing calculation/effect methods require `&mut Game`, so the live slot borrow must be released before invoking them. Reborrowing afterward can compare values but does not recreate actor/world causality or roll back prior effects. The new private-baseline comparator solves value equality only.

A useful **read-only** invocation-scoped observer could preserve call order and at-most-once forwarding around a proven native call site, with no commit receipt or write authority. That would not replace the current TConstruct retained-tool mutation. A reusable mutation service is not yet justified by a supported fully proven seam: the exact blocker is carrying a physical-slot/actor scope across mutable Game calculation while preserving the current consumer's distinct native-effect policy and the enclosing commit tail. Establish a guarded native phase contract and an actual target/borrow proof before deciding on that API.

Keep dormant armor record/end helpers excluded; they are in disabled-injections. Keep canonical saved-slot identity separate from live identity. No blanket comparator replacement follows from this investigation.

## Actual official plan probe: rejected before receiver binding

On 2026-11-05 at 20:38 UTC, the pinned official Mixins CLI attempted one `plan` against the pristine official Horizon export. The separate probe selected only `self.wear` in `Game::damage_held`, ordinal 0, require 1. Its helper read `held_slot` before and after calling the continuation exactly once and returned the unchanged result. It did not request mutation, cancellation, apply or Cargo execution.

The official planner terminated with exit 1:

```json
{
  "code": "E_UNSUPPORTED_CONTROL_FLOW",
  "message": "closures, async, try/?, unsafe and const blocks are unsupported injection contexts",
  "file": "crates/horizon-client/src/game.rs",
  "mod_id": "pluto_ranged_read_probe",
  "injection_id": "read-invocation-around-wear",
  "target": "horizon_client::game::Game<'_>::damage_held",
  "selector": "self.wear"
}
```

The target contains a `map_or` closure. This is an official planner capability failure, not a permission denial or a Rust type error. It does not establish whether receiver forwarding would otherwise compile. No alternate target, body rewrite or side-state workaround was attempted. The approved official Git checkout was clean before and after; both relevant native source files were byte-compared against its pristine export before and after. The continuous disk guard did not stop the probe; recorded minimum free space was 11,002,572,800 bytes.

Evidence hashes:

- Probe manifest: `aa9ee9ab9690b1bd3dc7633fa2820d5bf3b41415d778fd487f8e2a58232be3ab`.
- Probe helper: `0d0c9a1d3b12921df94291dbdf54d48bcdf284bc8be967527acac8805177959e`.
- Exact planner diagnostic: `b98db8b86f0068f1a048391c113d9a29bcd2b1abb93bb2aae5b0759d84e64b2b`.

The result narrows the next required boundary: official support for this native call context, or a separately reviewed exact-body observation adapter with full source/composition guards. Neither is an authorization to change TConstruct's bypass policy, freeze a different physical slot, add native effects, or expose general live-write authority. A successful future plan alone would still require actual generated-code/borrow and phase-order proof.

### Minimal probe inputs

These are diagnostic inputs only, not part of the Pluto package or an installed injection.

```toml
[mod]
id = "pluto_ranged_read_probe"
version = "0.1.0"
priority = 100
[compatibility]
horizon_revision = "624c4f6265c02296fbe8a80f029dd0d987fb94c0"
[[injection]]
id = "read-invocation-around-wear"
target = "horizon_client::game::Game<'_>::damage_held"
kind = "WRAP_OPERATION"
selector = "self.wear"
require = 1
ordinal = 0
helper = "wrap.rs"
bindings = [
  { name = "game", ty = "&mut Game<'_>", value = "$receiver" },
  { name = "stack", ty = "&ItemStack", value = "$arg0" },
  { name = "amount", ty = "u32", value = "$arg1" },
  { name = "original", ty = "impl FnOnce(&mut Game<'_>, &ItemStack, u32) -> u32", value = "$operation" },
]
```

```rust
fn read_invocation(
    game: &mut Game<'_>,
    stack: &ItemStack,
    amount: u32,
    original: impl FnOnce(&mut Game<'_>, &ItemStack, u32) -> u32,
) -> u32 {
    let _slot_before = game.held_slot();
    let result = original(&mut *game, stack, amount);
    let _slot_after = game.held_slot();
    result
}
```

Reproduction uses the official e8cd0aa Mixins CLI with `plan --workspace <pristine-official-624c-export> --mods <directory-containing-only-this-probe> --out <plan.json>`. It must not use a modified host or an unreviewed fork. A repeated plan was not needed for the recorded result.

A later [read-only native item-damage phase probe](native-wear-phase-plan-20261105.md) passed target/binding planning without Game access. Its narrower observation coverage does not resolve the mutation-authority or current TConstruct bypass questions above.
