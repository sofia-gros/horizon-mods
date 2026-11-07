# Read-only native item-wear phase: official plan result

## Result and scope

On 2026-11-05 at 20:58 UTC, one read-only official Mixins plan succeeded for `horizon_client::game::native_hooks::Game<'_>::native_item_damaged` with a noncancellable RETURN helper. The explicit unit result is returned unchanged. The helper receives only UUID, name, item name, and immutable shared reborrows of the amount and cancel parameters. It receives no Game, stack, world, physical-slot handle or mutable native field.

This is a **target/binding planning result**, not generated Rust type/borrow validation, runtime phase proof, or a new installed service. No apply, Cargo compilation/test, registry, consumer migration or production hook was added. The diagnostic helper is the entire experimental implementation; it is reproduced below. A new public registry was deliberately not added without a concrete consumer need.

The task's preservation constraints are unchanged: TConstruct's current native-wear bypasses remain bypasses, and machine metadata/notification/held publication order is untouched.

## Why this phase is narrower but useful

Official `game/enchant_effects.rs::wear` (lines 90–108) first calls `durability_loss`. When loss is positive and both acting UUID/name exist, it calls `native_item_damaged` with mutable points/cancel. It then chooses zero for cancellation or clamps points to a nonnegative value, and, for positive loss, calls the durability advancement trigger. The caller performs its own stack mutation and break handling later.

The native hook itself (game/native_hooks.rs lines 7578–7587) is an ordinary dispatcher statement. The RETURN helper observes parameters after this dispatch normally returns, at its position in the composed RETURN chain. It is not guaranteed to observe later RETURN modifiers, and it still runs on a prior cancellable HEAD exit under the official composition rules. Therefore it is not a receipt that every original dispatch listener ran, nor is the raw amount a final applied loss.

Native API 162 explicitly excludes creative wear, zero loss after Unbreaking, mob armor, dispenser tools and the LAN host. In the reviewed native caller it requires acting UUID/name. TConstruct's active ranged/melee retained-tool paths call `durability_loss` and intentionally bypass native `wear`; they are not suddenly covered by this diagnostic. Dormant dense-loss and armor-record declarations in `tconstruct/disabled-injections.toml` remain dormant.

The event's UUID/name identify the native event subject; they do not establish a world-generation or physical-slot capability. The event passes an item name, not full ItemStack identity. Cancellation and points describe this native policy phase only. No comparison with the new private-Wire comparator would supply the missing authority.

## Other alternatives and why they were not implemented

- The prior `damage_held` interior WRAP probe was rejected with `E_UNSUPPORTED_CONTROL_FLOW`; it was not retried.
- A direct RETURN on `durability_loss` was prepared but **not executed** after source inspection found opaque `world_stations!`/`world_items!` macros. Official `docs/standard-macros.md` allows only validated `format!`, `matches!` and `vec!` in body-rewriting hooks; `plan.rs` sets opaque-body macro preservation only for HEAD. This is a source-derived restriction, not a second recorded tool failure.
- A HEAD can preserve those opaque bodies but has no computed loss yet. A separate HEAD/RETURN registry or actor-keyed pending map would introduce state and nesting/lifecycle questions rather than create an invocation-owned token.
- An INVOKE bridge could ordinarily delegate without rewriting a body, but the official bridge validator rejects generic owner structs (`bridges.rs`, owner validation); Game has a lifetime parameter. This was source-inspected, not probed. A direct child-module wrapper around the method would not create authority either.
- Existing ranged/melee/projectile consumers already combine their item-ID filter and TConstruct edits under one native `held_mut` borrow after calculation. Factoring that block into a generic Option/filter/edit wrapper would merely preserve the same weak policy and add no authority boundary. No such wrapper was added.
- Holding a native slot borrow while invoking mutable Game calculation is not possible under normal Rust exclusivity. Releasing it and acquiring another late-bound slot does not bind the old actor/world/physical slot. Changing that policy or moving machine notifications would exceed mechanical extraction.

Thus a read-only official observation boundary is available at planning level, but no additional safe generalized equipment or machine commit API is established. If a concrete consumer later needs these facts, the next gate is actual applied generated-code checking plus native event-order tests, with explicit coverage and composition guards. Do not promote this plan to live write authority.

## Exact evidence

- Official Horizon: `624c4f6265c02296fbe8a80f029dd0d987fb94c0`.
- Official Mixins: `e8cd0aa38cffef49cb2e088ef716baa107148c02`.
- CLI result: exit 0, one injection, zero modules/crates/states/bridges, 6.581503864 seconds.
- Plan schema/transformer: 6 / 0.6.0. The CLI records rustc `1.95.0 (59807616e 2026-04-14)`.
- All 5,008 pristine input file hashes are recorded in the plan. The approved official Git checkout was clean before and after; game.rs, enchant_effects.rs and native_hooks.rs were also byte-compared with the pristine export before and after.
- Target function fingerprint: `9be83258e4b72f7a253801d7bb1a5f32703dcd85fc8779e361ad47edc56f2430`.
- Manifest SHA256: `d483bc274bb18ca45b529ebe08208c03c68d2c246b96b87707fbf0afc21b08be`.
- Helper SHA256: `a8c19b7714ccef37f2d269ad1c7d0c96cb50c9a8871979c8b0d2dcae732c2201`.
- Plan SHA256: `1a5a4395411f1c3affbc7d5668c92db508af91f78ced4a02c22bf5c340647b28`.
- CLI result log SHA256: `e5f87697b19431799d0e63869dd7b3b7229bedb0adb2a703f2253d3cc08187b9`.
- Continuous disk guard: threshold 10 GiB + 128 MiB; minimum/end free 11,178,459,136 bytes; no stop. No Cargo/CLI process remained after terminal status.

### Diagnostic manifest

```toml
[mod]
id = "pluto_wear_phase_read_probe"
version = "0.1.0"
priority = 100
[compatibility]
horizon_revision = "624c4f6265c02296fbe8a80f029dd0d987fb94c0"
[[injection]]
id = "read-native-wear-policy-result"
target = "horizon_client::game::native_hooks::Game<'_>::native_item_damaged"
kind = "RETURN"
helper = "observe.rs"
bindings = [
  { name = "value", ty = "()", value = "$value" },
  { name = "player", ty = "u128", value = "player" },
  { name = "name", ty = "&str", value = "name" },
  { name = "item", ty = "&str", value = "item" },
  { name = "amount", ty = "&i32", value = "&*amount" },
  { name = "cancel", ty = "&bool", value = "&*cancel" },
]
```

### Result-preserving diagnostic helper

```rust
fn read_native_wear_policy_result(
    value: (),
    player: u128,
    name: &str,
    item: &str,
    amount: &i32,
    cancel: &bool,
) {
    // Read-only result facts. No Game, mutable native field, stack or slot token.
    let _facts = (player, name, item, *amount, *cancel);
    value
}
```

Reproduction uses the pinned official Mixins CLI with `plan --workspace <pristine-official-624c-export> --mods <directory-containing-only-this-probe> --out <plan.json>`. The recorded diagnostic was not installed into the Pluto or TConstruct manifests.
