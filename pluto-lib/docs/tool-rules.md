# Evaluate a Tool's rules

A Tool component can give each item stack its own mining rules. PlutoLib evaluates the rules in order and falls back to Horizon's item definition when the stack has no Tool component. This is an experimental native bundle targeting Horizon `aa60eb76ea82bd66c29fea352e3ab5771e62ab26` and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`; it has not yet been built or tested in a game.

The reusable evaluator accepts plain Rust data and a callback that matches a block ID or `#`-prefixed tag against the block being mined:

```rust
use horizon_items::items::pluto_tool_rules::{evaluate, Rule};

let rules = [
    Rule { blocks: vec!["#example:mineable".into()], speed: None, correct_for_drops: Some(false) },
    Rule { blocks: vec!["minecraft:stone".into()], speed: Some(2.0), correct_for_drops: Some(true) },
];
let result = evaluate(&rules, 1.0, |name| {
    name == "#example:mineable" || name == "minecraft:stone"
});
assert_eq!(result.speed, 2.0);
assert!(!result.correct_for_drops);
```

The first matching rule that supplies a speed wins. The first matching rule that supplies drop correctness wins independently, so the answers can come from different rules. With no matching speed, the component's default speed is used; with no matching drop answer, the result is `false`.

The native adapter reads the actual `minecraft:tool` component and uses Horizon's block and tag matching. With no component, or a component it cannot parse, it leaves Horizon's result in place. Extra fields are not validated by this evaluator, so prepare components before using them. Use [Tool preparation](component-transport.md) before registering or changing a component. The adapter contains no material, modifier or broken-tool rules. Interaction with other native mods that intercept these methods, including Tinkers, is not yet verified.

The [independent demo](../examples/tool-rules-demo/mixins/pluto_tool_rules_demo/demo.rs) prepares a pickaxe with speed 2 for the pickaxe tag. The vanilla diamond pickaxe's item definition supplies a different speed, making it possible to distinguish component evaluation from fallback in the planned game check. The demo and target version are not yet game-verified.

## Try the demo

Stage `mixins/pluto_native_creative`, `mixins/pluto_transport_3a`, `mixins/pluto_component_display_3a`, `mixins/pluto_tool_rules`, and `examples/tool-rules-demo/mixins/pluto_tool_rules_demo` as sibling bundles with official Mixins. Build the pinned Horizon client, then set `PLUTO_DEMO_RESOURCES` to prepared resources and `PLUTO_DEMO_OVERLAY` to a new path outside them. Launch the client with `--resources` pointing to that overlay, as in the [native Creative demo](native-creative.md). The older bundle directory names are retained; their manifests identify the actual target revision.

In the planned check, select **Pluto Tool Rules** in Creative, obtain the pickaxe, and compare its mining speed and drops against the same item without the per-stack Tool component. A guest round trip and a Creative slot update must also be checked on the candidate host. No game result is claimed until those checks pass.
