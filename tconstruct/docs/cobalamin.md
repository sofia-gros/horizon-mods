# Worn Cobalamin attack and mining speed

This implements the uncovered attack/mining portion of the high-priority #379
Shell-material Cobalamin item. The source does **not** add ordinary movement
speed: its three effects are attack speed, item-use speed, and mining speed.
Use-item speed is separately implemented by PR #694 and is not changed here.

Source: TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, generated
`cobalamin.json`, `AttributeModule`, `EquipmentChangeWatcher` and
`ToolEvents.onBreakSpeed`.

## Covered

- Worn stock armor in its class's correct head/chest/legs/feet slot, including
  the existing saved Kobold shell material trait. Live material and fixed traits
  plus integer upgrades are resolved through the existing material resolver.
- Each piece separately multiplies attack speed by `1 + 0.05 * level` and the
  mining attribute by `1 + 0.15 * level`. The coefficient/level expression uses
  source float arithmetic before widening to double. Attack speed caps at 1024;
  the mining attribute caps at 100.
- The actual main-hand cooldown consumer is updated for native weapons and the
  existing TConstruct/Overwield path. Own transient per-slot attack attributes
  are refreshed after equipment changes and resource reload, including removal.
- Mining applies after tool conditional additions, including when mining bare
  handed or with a native item. It combines with the existing chest-Haste bridge
  before final speed multiplication, retaining double precision until the final
  float cast for Cobalamin. With no supported Cobalamin, the old path is retained.
- Source percent-style AttributeModule equipment hooks do not filter brokenness;
  broken worn pieces retain these attributes. Unknown definitions, missing
  resource snapshots, malformed levels/counts and opaque wire patches fail closed.
  Definition comparison is structural JSON, allowing whitespace-only changes.

## Limits

This does not add a Kobold acquisition route or prove live shell construction.
It does not connect held Cobalamin, custom armor classes, custom modifier modules,
external custom attributes, or the custom mining attribute's command interface.
The existing Haste baseline is already rounded to float; full mixed-attribute
ordering/rounding and custom armor-stat contributions are not reconstructed.
Other attack-speed modules and native clamps still follow existing hook order.

Only authoritative local/server actor state is used. Remote-client prediction
needs a separate actor-specific wire bridge and is left unchanged. Actual saved
world/reload, guest swaps, client prediction and LAN have not been exercised.

Brief static review and formatting only. Two source-rule regression tests were
added and the existing static Haste wiring assertions adjusted, but no tests,
builds, planner, CI, version checks or merges were run for this change.
