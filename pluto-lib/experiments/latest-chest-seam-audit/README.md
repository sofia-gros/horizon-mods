# Chest pick: known official seam constraints after the LOAD rejection

Scope: exact Horizon **41f822b659789780f58d32c63ddfb6887ea1b8bf**, Mixins **5a64af938ec86b00d53d16a408aabc0038fb3e57**. This is source-only analysis. Only the owned ItemStack LOAD has an actual diagnostic here: [2080080c](https://github.com/sofia-gros/horizon-mods/commit/2080080cd86f07f06a7d67f5e21a7d5f5cc4c632) records `E_UNSUPPORTED_LOCAL`, primitive-scalar-only LOAD. Alternative routes below have **not** been submitted to another planner.

## Final expression does not bypass the whole-target safety pass

`MODIFY_EXPRESSION_VALUE` can describe a concrete nonprimitive call result, so `Some(stack)` is not rejected merely because it is an Option. However, actual `plan.rs::validate_target` sets `rewrites_body = true` and `try_outside_value = false` for this kind. Its Safety traversal checks the entire target before the selected expression. `picked_stack` contains the outer `picked_item(...)?`, which this path rejects. An expression selector for the final `Some(stack)` does not narrow that safety traversal. Changing the kind alone is therefore not a supported workaround.

The original RETURN suffers the same outer-`?` restriction. MODIFY_VARIABLE allows a try outside the selected value, but LOAD explicitly admits only the listed primitive scalar types (`plan.rs` 713–727). Do not weaken the planner, claim selected IR exists, or retry an unchanged unsupported capability.

## The observed callers do not offer a simple supported result STORE

Exact immutable source reads find these three picked_stack call sites in two files; this is **not a repository-wide absence proof**. Verbatim excerpts and immutable links are retained in `observations.json`.

- `game.rs:17544`: local middle-click receives `if let Some(stack) = self.picked_stack(...)`. The result is a conditional pattern binding. `Scopes::visit_expr_let` calls `add_pattern(..., false, ...)`; `variable_sites` rejects destructuring/conditional bindings.
- `game.rs:6444`: creative hive-drop uses the same conditional `let Some(stack)` form in a chain. It is also not a plain supported local STORE.
- `game/lan_actions.rs:2866`: the block result is returned from a closure passed to `guest_pick_item`. That method later obtains `let stack = stack(g)?` inside its `as_guest` closure. Scopes marks declarations `in_closure`, does not select them through ordinary local declarations, and the selected value itself contains `?`. Moving policy there also adds caller-specific authority and from-entity distinctions; it is not a single native post-pick boundary.

`native_player_pick_item` is guest-side and cancellation-only. Its formal arguments contain no ItemStack result, and local middle-click does not pass through it. Reconstructing the picked stack or writing inventory in that event would move policy and authority rather than preserve the old result amendment.

## Initial STORE is not the old post-collection operation

The plain `let mut stack = ItemStack::new(item, 1)` is earlier than collection. Native code may then encode its components, extend with collected block-entity patches, and replace `stack.damage`/`stack.extra` from decode. Applying metadata color at that initial STORE would expose a changed stack to native component conversion and allow a later patch to override it; the original helper amended the completed stack afterward.

A paired early-stack/collected-patch edit is not a proven narrow equivalent: it changes metadata read count and timing, intermediates visible to other component hooks, and data flow through native encode/decode. Empty-patch, malformed/unknown data, include_data/banner/pot branches and all modeled fields would need a separate proof. No such paired workaround, cloned stack, TLS/global map, recursion bypass, or copied whole native body is proposed or enabled in this intake.

## Remaining requirement

The clean missing boundary is a supported synchronous post-collection owned result amendment with the same Game and formal position, preserving native early None and all other stack fields. The presently inspected official APIs/kinds do not establish that boundary. A supported official result hook or a separately justified capability change would be needed to implement it cleanly. Until an independently reviewed supported seam exists, keep the chest compatibility blocker explicit. Do not silently drop the feature or call mining-only integration/full compilation a complete Tinkers result.

The separately prepared mining-only and outer-tank intakes do not alter this chest RETURN/helper. Existing main, PRs, the official planner and prior evidence remain unchanged. No new plan, metadata, apply, Rust type gate, or native runtime was executed for this analysis.
