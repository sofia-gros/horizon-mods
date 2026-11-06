# TConstruct PR regrouping proposal, 2026-11-06

## Status and fixed inputs

This is a read-only consultation checkpoint, not a request to close or retarget PRs.
It proposes eight broad review groups. The final partition has not been approved.
No runtime code, PR state, branch protection, review, approval, merge setting or CI dispatch changed in this audit.

- Repository: `sofia-gros/horizon-mods` only.
- Separately verified main: `08afa992d5091e03360f3473ea93fd17aa793987`.
- Pluto v15 local checkpoint: `8715b1232c0fca133fe012f35830e2108e653d59`.
- Exact-tree remote checkpoint: [89744814](https://github.com/sofia-gros/horizon-mods/commit/89744814b8d536a117e82f8ea3c8cdc9798faa95), tree `267be30ceab0c59e8763a1e2de4db0f4fff83cf5`.
- The adjacent JSON freezes the 265 open PR head/base records read on 2026-11-06. These reads are paginated, not an atomic repository transaction. Recheck relevant refs before any future write.
- Official host and Mixins pins remain `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and `e8cd0aa38cffef49cb2e088ef716baa107148c02`. This audit did not access a user engine fork.

## What the graph establishes

There are 265 open PRs: 264 Draft and one non-Draft (#678).
195 PRs use the head branch of another open PR as their base, 67 use main, and three have bases not represented by an open PR.
The open-PR graph has 70 roots/components. The longest two chains contain 33 PRs each, ending at #939 and #941.

The unmatched bases are #770 and #832 on `fix/casting-state-preservation-379`, and #840 on `fix/drain-container-action-379`.
This means their base history needs inspection; it does not establish that those bases are lost, closed, merged or safe to discard.

A branch-name edge is not a proof of semantic dependency. PR base SHA fields are not the current main ref: for example, stale PR metadata can differ while the separately read main ref remains 08afa992.
The JSON keeps both values rather than silently replacing one.

Important chains include:
- Guide content: 23 PRs rooted at #693 and ending at #805.
- World-fluid content: 23 PRs rooted at #716 and ending at #932.
- #944 is the v13 foundation draft; #945 is based on it and contains the navigation repair.
- The 33-PR #675 to #941 chain crosses advancement, recipe and guide-adjacent work. A terminal cumulative tree must not be assigned wholly from its last title.
- The 33-PR #873 to #939 chain contains armor acquisition and later behavior work. Each increment still needs its own preservation mapping.

No individual PR has been finally assigned to a new group by title alone.
The per-domain audits must compare fixed commits, merge bases, direct changed paths and repeated patch content.

## Proposed eight groups

| Group | Review scope | Boundary rule |
| --- | --- | --- |
| 1. PlutoLib foundation | Shared APIs, native adapters, exact official pins, packaging, composition guards and validation tools | No TConstruct content or actor authority is moved merely to make an API reusable. |
| 2. Items, materials and data | Tool/part structures, identifiers, material data, codecs and acquisition data | Generic representation services may be shared; content/schema/ID policies remain in the mod. |
| 3. Recipes, stations and advancement | Matching, material acquisition, recipe ordering, menu framing, station actions, discovery/progress | Shared pure mechanics are distinct from actual crafting authority and result/remainder consumption. |
| 4. Tools and combat | Melee, ranged, harvesting, tool modifiers and their acquisition | Preserve source feature behavior, call order and native actor ownership. |
| 5. Armor, defense and movement | Worn effects, shields, armor modifiers, movement and defenses | Shared input/state helpers do not certify complete native effect lifetimes or gameplay. |
| 6. Machines and storage | Smeltery, Melter, casting, fuel, tanks, transfers, inventory and persistence policies | Storage representation/quantity plans are not save, sync, rollback or world-effect authority. |
| 7. World, fluids and rendering | Slime/world content, fluid sampling/contact, meshing and effects integration | Read-only sampling, native type checks, CPU equivalence and live effect authority are separate gates. |
| 8. Guide, UI and LAN | Guide content/navigation, text/layout, retained presentation, request/reply integration | Input admission, retained ownership, codec compatibility and transport authority must remain separate. |

These are review groups, not eight immediate merge approvals.
Cross-cutting commits can be decomposed by owned changes with explicit mapping, or retained as a small dependency shared by groups.
Do not duplicate one source delta in two groups.

## Stage 2 commonization: what is actually proved

The complete v15 Pluto library package passed official plan/apply, all 12 mandatory composition guards, and actual host lib/bin type checks.
The proof observes real recompilation of changed API/client targets, verifies package/dependency identity, and seals all 5,047 applied source files plus generated inputs.
See [v15 proof](shared-foundation-v15-candidate-20261106.md) and its JSON receipt.

That does not certify the complete TConstruct consumer, unmerged feature PRs, linked execution, gameplay, GPU behavior or all LAN behavior.
The full consumer remains blocked by the old materials-book response hook's conflict with the shared payload observer contract.
The navigation-only repair #945 has focused helper/plan proof, but not its complete host type/UI proof.
The ordinary install path must still use the guarded staging procedure; a packaged archive alone does not execute the Python composition guards.

The following post-v15 units are separately published and must not be relabeled as part of the v15 composition proof:
- [Slot insertion adoption](https://github.com/sofia-gros/horizon-mods/commit/375fa4aef1e70f17e5d79d2e5bbf94bbcb87192f): 42 focused native tests; preserves caller policy.
- [Fluid slot quantity adoption](https://github.com/sofia-gros/horizon-mods/commit/b0e911b5a294c430a471e168f86c7ca0ed7152f4): 144 actual module tests, including 23,220 differential value/callback comparisons.
- [Java ordering extraction](https://github.com/sofia-gros/horizon-mods/commit/b4be4b1347ad5168fc63a6227f9759ae55e30041): 8 Rust tests and 1,311 legacy comparisons; historical Java golden vectors, no live javac run.
- [Menu-word framing extraction](https://github.com/sofia-gros/horizon-mods/commit/d1c52d28a56696b5364246c03abaa658b0c2e454): 11 Rust tests, 7,600 malformed and 390 encode/roundtrip scenarios; no live-network proof.

The actual Game borrow probes and renderer CPU proof remain unexecuted.
The local direct-rustc alternative has no complete historical dependency/input seal or established effective RAM bound.
Manual CI preparation is separate work, not a successful run.

## One ledger, two kinds of rows

Keep PR rows and API/behavior contract rows separate. Their counts are not completion percentages.

Each PR row needs:
- PR number, original head/base refs and exact SHAs, actual merge base, parent PR if known.
- Its direct increment and cumulative changes, changed paths, equivalent patches already present elsewhere, and remaining differences.
- Proposed target group and preservation mapping. Mark ambiguous or mixed content rather than guessing.
- Dependencies, shared paths and conflicting edits, test evidence for that exact cutoff, and the future superseding PR only after it exists.

Each contract row needs:
- Concrete production caller and its source/hash evidence.
- Existing shared service, retained mod policy, and any missing owner/lifetime/transaction boundary.
- Exact verified scope and separately unrun/failed/blocked scope.
- Conditions for retiring a wrapper; do not infer deletion permission from similar names or a passing library test.

Current separate domain evidence:
- [Recipe/advancement contracts](https://github.com/sofia-gros/horizon-mods/blob/69e3ddc6f220def799031813ed025b19842de1f9/pluto-lib/docs/recipe-advancement-boundary-inventory-20261106.md): 28 boundaries and 58 source records, not 28 PRs; 17 shared services, 8 retained policies and 3 unresolved boundaries, with zero unconditional removals.
- [Storage contracts](https://github.com/sofia-gros/horizon-mods/blob/016d867e7c693e31a462d97e8335e00c30b4c51c/pluto-lib/docs/storage-reorganization-inventory-20261106.md): 14 contracts, 31 mod source/proof anchors and three official source checks. v15 and the two later consumer overlays are separate.
- Guide and World PR/contract ledgers are being reconciled separately. Their source review must not promote available Pluto services into completed consumer migration.

## Safe integration order

1. Freeze the current refs and preserve all original source/proof checkpoints. Keep exact original-to-new commit/tree mappings; snapshot publication is not original Git history.
2. Complete the per-PR direct-diff, merge-base and duplicate-content ledger. Resolve the three unmatched bases and cross-group chains before selecting commits.
3. Build a new integration line from the then-current main, preserving shared foundation first. The current #944 diff is not Pluto-only, so do not merge it blindly as the foundation layer.
4. Bring each source delta across once, retain necessary mod policies, and use shared APIs only where equivalence or an explicitly reviewed correction is proved.
5. Keep edits to manifests, transaction helpers, Game/UI ownership and common CI in one coordinated integration path. Run all guards on the complete composed plan, not only separately passing subplans.
6. Review and validate each group at its final head: source/fixture tests, complete official plan/apply/type gates, and the group's relevant save/LAN/runtime tests. Record failures and unrun cases explicitly.
7. Merge only groups whose dependencies and required gates are satisfied. Leave blocked portions Draft or separate from runtime activation; do not disable existing features to obtain a green plan.
8. Only after the preservation map and replacement review are complete, and regrouping is authorized, mark superseded old PRs and close them. Keep traceable links and original source refs/history preservation.

## Main risks to review

- Cumulative stacked PRs can duplicate earlier patches or accidentally drop later features when squashed by terminal branch.
- Similar filenames or passing generic fixtures do not establish feature equivalence.
- Library/type proof is not a substitute for full consumer composition, actor authority, storage durability or runtime behavior.
- Shared manifests, transaction.rs, book/Game ownership and workflow edits are likely collision points.
- The guide's existing retained-overlay refresh, cover sentinel and legacy reply envelope need explicit compatibility proof; merely wiring an observer is insufficient.
- Automatic output/remainder pairing still needs a real same-operation owner and consuming allocation contract.
- World effect instance identity/save and synchronous contact ownership remain distinct from read-only fluid sampling.
- Tests/proposals excluded from packaging must stay excluded. Original native/official inputs and guard scope must not be weakened to manufacture a pass.

## Validation of this audit

The JSON was parsed and checked for 265 unique PR numbers, valid parent references, no open-graph cycles and internally consistent counts.
A fresh three-page read found no head/base-ref/draft changes from the earlier saved snapshot.
Only two documentation files are proposed for publication on the fixed v15 tree. This audit does not claim a new native test result or any PR action.
