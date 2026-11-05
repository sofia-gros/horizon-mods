# Reinforced and Unbreakable ordinary armor wear

Partial high-priority #379 durability implementation, stacked on Dense armor
wear #929, Tanned #925 and retained writeback #923.

The scoped stock armor path now admits complete canonical Reinforced and
Unbreakable definitions. Canonical Netherite is also admitted among no-tool-damage
modifiers so the ordinary Unbreakable prerequisite combination can participate.
Other active modifiers must satisfy the existing bounded guards.

## Source order and arithmetic

- Tanned priority200 runs first.
- Unbreakable priority125 cancels all remaining points when the ordinary EMPTY
  cause is not in the live `durability/bypass_reinforced` tag.
- Dense and Reinforced both default to priority100. Source `ModifierNBT.Builder`
  preserves insertion order for equal priorities. Saved upgrade order is used
  first, followed by live material/tool trait order; merging retains the first
  position. An ambiguous two-upgrade order fails closed before writeback.
- Reinforced merges integer material levels with saved quarter-level progress,
  builds the started logical level, then adds the negative fractional correction
  in float. The logical level chooses the <=5 or >=6 source module. The source
  multiplication/addition order is preserved.
- Reinforced is disabled whenever Unbreakable is present, including when its own
  hook was bypassed. Dense still operates for a bypassed cause.

Each partial reduction draws once per point left by earlier hooks. Zero/certain
prevention does not draw. Validation at record/commit never rerolls. Final native
loss is committed exactly once through #923; native item-damage callbacks and
advancement reporting still happen after the prevention calculation.

The existing live tag resolver now exposes membership for this shared tag and
recognizes the complete canonical Glowing definition, its default member. This
is registry evidence for tag resolution, not enabling Glowing as an active worn
modifier. Nested/replace/remove handling remains shared with Tanned. Unknown
registration or malformed tags fail closed. Changed tag membership can therefore
bypass ordinary Reinforced/Unbreakable using EMPTY without a hardcoded cause list.

## Limits and review

Solar Powered, Overslime, Barkskin and other unsupported hooks remain outside
this slice; stored positive Overslime is still rejected. Arbitrary native
enchantments, source/native unbreakable flags, opaque records, wrong slots,
broken gear and remote/creative actors remain excluded. Source break animation,
generic held wear and native batched health-event ordering remain as in #923.
The old shared fixed Dense-first helper on unrelated paths is not changed.

Source: TConstruct f4e77d43f39ba1047bf93ae816b7e1beaab514c2 (MIT), reinforced.json,
unbreakable.json, netherite.json, glowing.json, IncrementalModifierEntry,
ModifierNBT.Builder, ToolStack and ReduceToolDamageModule. Canonical definitions
retain the source copyright notice and bundled license reference.

Brief static source/API and whitespace review only. Five pure regression cases
added but not run. Tests, builds and planner execution are left to the separate
batch integration pass. No CI inspection or merge. Gameplay, save/reload,
dedicated and guest/LAN remain unverified. No native extension or
provenance-blocked fluid code is included. The base defense chain still requires
native signed-protection 09f0179db and upper-bound 5e47547e2.
