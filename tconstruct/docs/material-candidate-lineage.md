# Material, equipment and world candidate lineage

This is a source-integration inventory, not a test or release status. PR heads
are immutable inputs for the dedicated test owner. A Git ancestor means the
commit is present; a ported change is identified separately and must not be
applied twice. All entries below remain subject to integration review and the
actual target's apply, type and game gates.

| Scope | Candidate | Relationship and remaining work |
| --- | --- | --- |
| Ordered ranged stats and upgrades | #291 `88edbd97244c3716e52d7efd9248800f9b809040` | Based on #289, #286, #282 and #279; includes the #282 test-lifetime correction. Power and Blindshot are ports of #109 and #133/#134; do not replay those implementations. #288 is closed as a duplicate. Solid from #86 and #279 must not both be replayed. |
| Island and planted Ender trees | #242 `990c4cf4` | Contains #226, #227, #230, #231, #236, #237, #238 and #240 as ancestors. #241's tint isolation is on a separate branch and must be reconciled; #242 alone is not evidence of that isolation. |
| Resource blocks and tags | #188 `765fb2dd` | Contains #178, #181, #183 and #186; #180 is separate from #188 and needs integration. #168, #173 and #141 are separately based on main and need independent review. |
| Gilded and slot rules | #165 `18a27664` | Contains #123, #124 and #128. Verify its meaning in a unified candidate before replaying individual PRs. |
| Snow Boots | #159 `b6a5076e` | Contains #156 and #158. Preserve both walking and placement behavior when integrating. |
| Step Up and older ranged chain | #164 `f6c2fe5c` | Contains #163 and the older #109/#133/#134 chain; reconcile source changes with #291 rather than replaying duplicate Power or Blindshot. |
| Other independent candidates | #136, #139, #140, #150, #161 | Different branches need source-level comparison, including overlap with the old ranged chain. Inclusion in an eventual candidate is not inferred from matching titles. |
| Bouncy | #273 and related #243/#245/#247/#255/#258/#264 | Landing hooks remain narrowly deferred after official plan failures. Preserve the evidence; do not treat submitted code as working bounce. |

A read-only merge preview of #291 and #242 reported conflicts in
`tconstruct/compatibility.json`, `tconstruct/docs/compatibility.md`, and
`tconstruct/mixins/tconstruct/horizon-mixin.toml`. The shared target and
manifest need coordinated resolution with their owner; this file does not
claim one buildable unified head. In particular, integrating #241's isolated
tint path and choosing the formal target must be explicit. The latest fluid
splash branch is separately owned and must be reconciled by its owner.

The #286 formal test report identified E0716 in crossbow test inputs. #291
contains the proposed lifetime correction; the prior pass was a disposable
old-host diagnostic, not a pass of #291. #283's later formal apply reports an
`E_INVALID_HELPER` in the separately owned splash path; its proposed return-type
explanation is unconfirmed and must be resolved before calling the combined
apply successful.

## Equipment integration after #301

The following is a source inclusion inventory for the follow-up branch. It is
not a substitute for the exact-head tests. The prior #298 and #301 heads remain
fixed for their ongoing checks.

| Plan | Included by ancestry or port | Still separate or incomplete |
| --- | --- | --- |
| F02 | Material trait fallback #117/#119 and ordered stats #251/#256/#260/#262/#267/#271/#275/#279 are ancestors. Fixed tool-definition traits #152 are ported. | Skull/overslime armor #107/#111, Wool #120, slime chest definition #121, Ductile toughness #136 and Forming armor #142 require individual source comparison and integration. |
| F05 | Actor-aware Raging/Airborne shooting #91/#140 are ancestors via #301. Raging melee #96 is ported with the actor turn and current ordered stat scale. Overwield ranged #102 and Spiny velocity #150 are ported. | Melee, guest actor identity and stat ordering await exact-head verification; Spiny self-damage remains deferred. Do not replay the listed old commits. |
| F06 | Vintage/Heavy/Featherweight and Solid/Entwined armor #130/#131/#132/#138, broken gravity #155 and Step Up behavior #163 are ancestors. Snow Boots #156/#158 and Golden armor #161 are ported. | Step Up recipe #164, Speedy #139, and other omitted armor candidates need source comparison. Bouncy remains narrowly deferred. |
| F12 | Power and Blindshot were ported in #289; ordered upgrades #267/#279 and splash fixes through #298 are ancestors. | Gilded and slot transactions #123/#124/#128/#165 and separately owned current fluid changes need reconciliation, not blind replay. |
| F14 | Island/vegetation and planted propagule candidate #242 is an ancestor, including #191/#200/#204/#206/#208/#212/#215/#218/#220/#223/#226/#227/#230/#231/#236/#240. Unsupported terrain tint remains absent. | Ender island marker world boundary, slime fluids and custom mob registration remain narrow deferrals; all integrated generation requires target game checks. |
| F15 | Island source candidate #242 is an ancestor. | #141 Cobalt loot, #168 cluster harvest, #173 slime loot, #178/#180 material tags and #181/#183/#186/#188 storage blocks are not ancestors; compare actual source before replaying, since product effects may be independently ported. |

A port means equivalent intent was incorporated into this branch, not that it
has passed official plan/apply/type or gameplay. #295 is present by cherry-pick
rather than Git ancestry. The #298 package source-identity check applies to its
fixed Pluto tree; these TConstruct changes need their own exact-head gates.

The follow-up after #304 ports Vintage/Gilded #123/#124/#128/#165 and Speedy
#139, and carries the source Cobalt sequence #141, cluster tool tag #168,
ingot/nugget registrations #178/#180 and Pig Iron/Knightmetal block series
#181/#183/#186/#188. Source loot #173 was already byte-identical in the
candidate and was not replayed. Existing fluid ability and Blindshot guards are
preserved during the Gilded merge. #142 Forming and #164 Step Up recipe remain
separate pending comparison. Package, target apply, type and game behavior of
this combined source remain unverified.

The subsequent target-integration candidate ports #268's named Ingredient
field for the declared 0c host and #305's narrower guest splash completion
hook. The other #305 dependencies are not implied to be included. Both fixes
are source-level; the 0c apply/type/game results must be checked on this exact
head. The previous #298 official apply stopped at guest packet RETURN.

The subsequent follow-up ports Forming #142 together with the previously
separate Heavy/Wool knockback and Ductile toughness contributions, and ports
Step Up recipe #164 without replacing the existing #163 equipment behavior.
The source stat and recipe effects must be checked on the exact integration
head; this inclusion does not imply formal apply or gameplay success.

The next follow-up ports skull armor #107, slime-material Overslime #111 and
slimy chestplate definition resolution #121. The material capacity is combined
with the later ordered Overlord copy rather than restoring the older capacity
formula; focused tests and all target checks remain pending. Wool #120,
Ductile #136 and Forming #142 are represented in the #309 armor calculation.

The subsequent F02/F05 follow-up ports #98 and #101 together, including the
intermediate broad-speed calculation, live Overslime reads and the area-harvest
spend. The #82 Airborne penalty boundary and later ordered stat calculations are
retained. These paths still need formal apply and gameplay checks; ordinary
single-block capacity spending and other Overslime consumers remain incomplete.

The reconciled candidate uses an after-dispatch `native_destroy_block` hook:
permission and mod cancellation checks happen before spending, while block
removal follows. Resolved item tags and the broken-state guard fail closed,
and actor-scoped area state prevents repeated spends. Focused plan/apply and
local stub tests passed; integrated host build, creative and gameplay remain
unverified. Other capacity consumers remain open.
