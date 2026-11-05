# Tanned ordinary armor wear

Partial high-priority #379 durability-pipeline implementation, stacked on retained
armor writeback #923 and live Tanned tags #916.

Tanned is now admitted by the armor writeback eligibility guard only when its
complete canonical definition and reachable cause tags resolve. The same shared
live evaluator runs at a new wrapper around `Game::wear`'s `durability_loss`
operation. This is a macro-free supported call boundary. Reduction happens before
native item-damage callbacks and advancement reporting, not at the later slot
commit. Native gear and wear outside the armor scope pass through unchanged.

Ordinary armor loss uses `tconstruct:empty` as its source cause. The canonical
first range caps non-bypassed loss to one; the second range sets secondary-cause
loss to zero even if the first range is bypassed. Live additions/removals/replaces
and nested cause tags are handled by #916. Positive material and saved Tanned
levels merge once. Every other active modifier must still be one of #923's exact
14 no-tool-damage definitions. A positive probe validates the Tanned definition
and tags even when the actual native loss is zero.

The final native loss is recorded and written to the unchanged equipped piece by
#923. A zero result does not change damage or broken state. Multiple armor pieces
are treated independently; Tanned does not cap a wearer-wide sum. Broken items,
creative/remote actors, wrong slots, changed definitions, unresolved tags,
opaque/malformed records and native-enchanted gear remain excluded. Unknown tag
leaves fail closed rather than being guessed absent.

This connects the ordinary Tanned armor path, including canonical leather armor
traits when combined only with supported modifiers. It does not restore Dense,
Reinforced, Unbreakable, Solar Powered, Overslime, Barkskin or the generic held
wear pipeline. Positive stored Overslime is still excluded. Source break animation,
arbitrary reentrant mod callbacks, native batched health-event ordering and the
remaining #923 limitations are unchanged.

Source: TConstruct f4e77d43f39ba1047bf93ae816b7e1beaab514c2 (MIT), `tanned.json`,
`ToolDamageRangeModule`, `ToolDamageUtil`, and the existing live cause-tag evaluator.
No native extension or provenance-blocked fluid code is added. The base defense
chain still requires unpublished native signed-protection 09f0179db and
protection-upper-bound 5e47547e2.

Brief static source/API and whitespace review only. Existing pure Tanned tag and
retained-writeback regression cases are available for the combined integration
pass; no tests, build or planner were run for this slice. CI was not inspected
and nothing was merged. Gameplay, save/reload, dedicated and guest/LAN remain
unverified.
