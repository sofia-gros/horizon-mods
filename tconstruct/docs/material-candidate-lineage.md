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
