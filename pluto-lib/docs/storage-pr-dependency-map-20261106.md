# Fixed storage PR dependencies and overlap

Read-only continuation of the [14-contract storage inventory](https://github.com/sofia-gros/horizon-mods/blob/016d867e7c693e31a462d97e8335e00c30b4c51c/pluto-lib/docs/storage-reorganization-inventory-20261106.md). This report changes no PR state, source, manifest or runtime behavior. No native build/test or discarded engine-fork access occurred.

## Frozen scope

Exactly the70 machines/storage PRs from the prior assignment are joined to the [265-row metadata snapshot](https://github.com/sofia-gros/horizon-mods/blob/4bf5fe7d06a08ae72330dea44ddfc445471f073a/pluto-lib/docs/pr-reorganization-snapshot-20261106.json), observed2026-11-06 04:39:33UTC. All70 recorded heads remain identical to the earlier audit. Metadata inputs and all referenced MOD commit objects were checked.

`reported_base_sha` is the REST PR object’s `base.sha`, not necessarily an ancestor, the current named branch tip, or the parent PR head. `actual_merge_base_sha` is independently computed from that value and the fixed head. The separate fixed main is `08afa992d5091e03360f3473ea93fd17aa793987`.

## Findings

- 46 declared parent-PR edges,21 `main` labels and3 unmatched historical base refs. All46 reported bases match the parent snapshot head.

- **23 heads are already ancestors of fixed main.** Preserve their historical evidence and do not blindly reapply their changes. This does not prove latest-host compatibility of their complete original branches.

- **6 REST bases are not ancestors:** #678,#679,#695,#696,#701,#704. Their actual merge-bases and base-only commits are recorded separately. This is graph evidence, not a reproduced missing-fix defect.

- No exact-head duplicate groups, no equal stable whole-incremental-diff patch IDs, and no equal stable patch IDs among the89 nonmerge incremental commits were found within this70-PR scope. There are97 unique incremental commits total;8 merge commits were deliberately not flattened for the single-commit comparison. This is not a semantic-equivalence proof.

- **65 paths overlap** across incremental diffs. `source-machine-runtime.rs` is touched by51 PRs. A shared file is an integration hotspot, not evidence the features are duplicates.

- **19 branches** contain additional paths when viewed cumulatively from their merge-base with current main rather than their reported-base merge-base. Importing a whole branch can therefore pull unrelated prerequisite changes.

- #705 and #714 contain nonofficial engine declarations in MOD metadata. No engine-fork content was accessed; these remain official-target recovery requirements, not newly verified heads.

## Graph table

Hashes are shortened for reading; the [JSON ledger](storage-pr-dependency-map-20261106.json) contains complete SHA values, exact commit lists, diff SHA-256, stable patch IDs, incremental paths, cumulative-only paths, inherited PR heads and source-contract associations. `Main` means head already present in fixed main; it is not a validation mark.

| PR | Head | REST base | Actual merge-base | Parent PR | Main | Incremental files |
|---|---|---|---|---|---|---|
| [#631](https://github.com/sofia-gros/horizon-mods/pull/631) | 75531f4bd2f4 | 8cf729ff4d7b | 8cf729ff4d7b | — | no | 24 |
| [#678](https://github.com/sofia-gros/horizon-mods/pull/678) | a7f2e207c2da | ddc324307deb | 40633d44e5f1 | — | no | 5 |
| [#679](https://github.com/sofia-gros/horizon-mods/pull/679) | da95d7a9224b | ddc324307deb | 40633d44e5f1 | — | no | 15 |
| [#680](https://github.com/sofia-gros/horizon-mods/pull/680) | df0856097f06 | ddc324307deb | ddc324307deb | — | no | 7 |
| [#682](https://github.com/sofia-gros/horizon-mods/pull/682) | a6ac9c17916f | 10ad8a4fc732 | 10ad8a4fc732 | — | no | 11 |
| [#685](https://github.com/sofia-gros/horizon-mods/pull/685) | 6ce1fcc25233 | af47e66842eb | af47e66842eb | — | no | 5 |
| [#688](https://github.com/sofia-gros/horizon-mods/pull/688) | f213eb1b7b03 | da95d7a9224b | da95d7a9224b | 679 | no | 12 |
| [#689](https://github.com/sofia-gros/horizon-mods/pull/689) | 7529a45967ef | f213eb1b7b03 | f213eb1b7b03 | 688 | no | 9 |
| [#695](https://github.com/sofia-gros/horizon-mods/pull/695) | 56df22558d80 | 19378cf3d5d6 | c709bf33b5c2 | — | no | 9 |
| [#696](https://github.com/sofia-gros/horizon-mods/pull/696) | 3796b0dde80d | 19378cf3d5d6 | 10ad8a4fc732 | — | no | 9 |
| [#700](https://github.com/sofia-gros/horizon-mods/pull/700) | bf0333ea1118 | 6d980fa32cf9 | 6d980fa32cf9 | — | no | 5 |
| [#701](https://github.com/sofia-gros/horizon-mods/pull/701) | 2670648eb206 | 6d980fa32cf9 | 19378cf3d5d6 | — | no | 10 |
| [#703](https://github.com/sofia-gros/horizon-mods/pull/703) | e1c56d06fe1a | 6d980fa32cf9 | 6d980fa32cf9 | — | no | 6 |
| [#704](https://github.com/sofia-gros/horizon-mods/pull/704) | 6ea90e36641c | 6d980fa32cf9 | bdd4ffe6287f | — | no | 6 |
| [#705](https://github.com/sofia-gros/horizon-mods/pull/705) | d6ad95b0e58e | 6d980fa32cf9 | 6d980fa32cf9 | — | no | 27 |
| [#710](https://github.com/sofia-gros/horizon-mods/pull/710) | e237df3ebf28 | 6d980fa32cf9 | 6d980fa32cf9 | — | no | 4 |
| [#714](https://github.com/sofia-gros/horizon-mods/pull/714) | dbfd2c632c33 | 6d980fa32cf9 | 6d980fa32cf9 | — | no | 30 |
| [#719](https://github.com/sofia-gros/horizon-mods/pull/719) | 44b9f8a8c0b4 | 6d980fa32cf9 | 6d980fa32cf9 | — | no | 5 |
| [#723](https://github.com/sofia-gros/horizon-mods/pull/723) | 4abf10da08db | 6d980fa32cf9 | 6d980fa32cf9 | — | no | 4 |
| [#728](https://github.com/sofia-gros/horizon-mods/pull/728) | 87f6cccf85d9 | 6d980fa32cf9 | 6d980fa32cf9 | — | no | 5 |
| [#730](https://github.com/sofia-gros/horizon-mods/pull/730) | 91828e36ddda | 4abf10da08db | 4abf10da08db | 723 | no | 4 |
| [#736](https://github.com/sofia-gros/horizon-mods/pull/736) | 041a15ab90d9 | 6d980fa32cf9 | 6d980fa32cf9 | — | no | 5 |
| [#740](https://github.com/sofia-gros/horizon-mods/pull/740) | be29e8ed903c | 041a15ab90d9 | 041a15ab90d9 | 736 | no | 4 |
| [#747](https://github.com/sofia-gros/horizon-mods/pull/747) | 2a9b7157e9c1 | be29e8ed903c | be29e8ed903c | 740 | no | 4 |
| [#752](https://github.com/sofia-gros/horizon-mods/pull/752) | 03d0fbd374b5 | 041a15ab90d9 | 041a15ab90d9 | 736 | no | 4 |
| [#756](https://github.com/sofia-gros/horizon-mods/pull/756) | c42c247a76dd | 6d980fa32cf9 | 6d980fa32cf9 | — | no | 5 |
| [#759](https://github.com/sofia-gros/horizon-mods/pull/759) | 79ac860df014 | c42c247a76dd | c42c247a76dd | 756 | no | 5 |
| [#762](https://github.com/sofia-gros/horizon-mods/pull/762) | 83147ab209e1 | 91828e36ddda | 91828e36ddda | 730 | no | 3 |
| [#766](https://github.com/sofia-gros/horizon-mods/pull/766) | 30ff170b90b7 | 87f6cccf85d9 | 87f6cccf85d9 | 728 | no | 3 |
| [#770](https://github.com/sofia-gros/horizon-mods/pull/770) | ebc41e57f8e8 | ace3fb1bdc3f | ace3fb1bdc3f | — | no | 5 |
| [#773](https://github.com/sofia-gros/horizon-mods/pull/773) | 4948cc842b18 | ebc41e57f8e8 | ebc41e57f8e8 | 770 | no | 5 |
| [#779](https://github.com/sofia-gros/horizon-mods/pull/779) | 7c5054d9f028 | 6d980fa32cf9 | 6d980fa32cf9 | — | no | 5 |
| [#780](https://github.com/sofia-gros/horizon-mods/pull/780) | 7caaa895d4ce | 7c5054d9f028 | 7c5054d9f028 | 779 | no | 5 |
| [#785](https://github.com/sofia-gros/horizon-mods/pull/785) | 83ffb8e70f93 | 7caaa895d4ce | 7caaa895d4ce | 780 | no | 5 |
| [#789](https://github.com/sofia-gros/horizon-mods/pull/789) | 46769bd9065e | 83ffb8e70f93 | 83ffb8e70f93 | 785 | no | 3 |
| [#792](https://github.com/sofia-gros/horizon-mods/pull/792) | f69c0c1fb183 | 46769bd9065e | 46769bd9065e | 789 | no | 5 |
| [#794](https://github.com/sofia-gros/horizon-mods/pull/794) | f1d894d9a77d | f69c0c1fb183 | f69c0c1fb183 | 792 | no | 3 |
| [#795](https://github.com/sofia-gros/horizon-mods/pull/795) | 97cd9252931f | f1d894d9a77d | f1d894d9a77d | 794 | no | 5 |
| [#807](https://github.com/sofia-gros/horizon-mods/pull/807) | 38d5ef5aae36 | 30ff170b90b7 | 30ff170b90b7 | 766 | no | 4 |
| [#810](https://github.com/sofia-gros/horizon-mods/pull/810) | 1bdd4fbb7f7f | 83147ab209e1 | 83147ab209e1 | 762 | no | 3 |
| [#813](https://github.com/sofia-gros/horizon-mods/pull/813) | b5546433c609 | 4948cc842b18 | 4948cc842b18 | 773 | no | 4 |
| [#816](https://github.com/sofia-gros/horizon-mods/pull/816) | dbf82153c1b9 | 2a9b7157e9c1 | 2a9b7157e9c1 | 747 | no | 4 |
| [#819](https://github.com/sofia-gros/horizon-mods/pull/819) | ba875f8401c1 | dbf82153c1b9 | dbf82153c1b9 | 816 | no | 5 |
| [#820](https://github.com/sofia-gros/horizon-mods/pull/820) | 4fa6c2501d91 | 97cd9252931f | 97cd9252931f | 795 | no | 5 |
| [#822](https://github.com/sofia-gros/horizon-mods/pull/822) | 4457b401d5ed | b5546433c609 | b5546433c609 | 813 | no | 5 |
| [#826](https://github.com/sofia-gros/horizon-mods/pull/826) | 85e4545a6a6e | 38d5ef5aae36 | 38d5ef5aae36 | 807 | no | 8 |
| [#827](https://github.com/sofia-gros/horizon-mods/pull/827) | 89fe96d38b14 | 85e4545a6a6e | 85e4545a6a6e | 826 | no | 5 |
| [#832](https://github.com/sofia-gros/horizon-mods/pull/832) | a5f9e6c1313e | ace3fb1bdc3f | ace3fb1bdc3f | — | yes | 5 |
| [#834](https://github.com/sofia-gros/horizon-mods/pull/834) | d6b710fdd9b2 | a5f9e6c1313e | a5f9e6c1313e | 832 | yes | 2 |
| [#838](https://github.com/sofia-gros/horizon-mods/pull/838) | 177a77db5edc | d6b710fdd9b2 | d6b710fdd9b2 | 834 | yes | 2 |
| [#840](https://github.com/sofia-gros/horizon-mods/pull/840) | 51a07aebb604 | ee7c0bf297fd | ee7c0bf297fd | — | yes | 5 |
| [#841](https://github.com/sofia-gros/horizon-mods/pull/841) | 7b55f4882ff0 | 177a77db5edc | 177a77db5edc | 838 | yes | 6 |
| [#844](https://github.com/sofia-gros/horizon-mods/pull/844) | 2d09578cc13e | 7b55f4882ff0 | 7b55f4882ff0 | 841 | yes | 7 |
| [#846](https://github.com/sofia-gros/horizon-mods/pull/846) | 8647bc5d2a6a | 2d09578cc13e | 2d09578cc13e | 844 | yes | 7 |
| [#848](https://github.com/sofia-gros/horizon-mods/pull/848) | 50c0136a8a66 | 8647bc5d2a6a | 8647bc5d2a6a | 846 | yes | 5 |
| [#849](https://github.com/sofia-gros/horizon-mods/pull/849) | e678dc43200e | 2d09578cc13e | 2d09578cc13e | 844 | yes | 5 |
| [#852](https://github.com/sofia-gros/horizon-mods/pull/852) | 698089600d55 | 51a07aebb604 | 51a07aebb604 | 840 | yes | 6 |
| [#853](https://github.com/sofia-gros/horizon-mods/pull/853) | 839137ac92f7 | 698089600d55 | 698089600d55 | 852 | yes | 7 |
| [#856](https://github.com/sofia-gros/horizon-mods/pull/856) | 3cb745b58cfc | 839137ac92f7 | 839137ac92f7 | 853 | yes | 6 |
| [#857](https://github.com/sofia-gros/horizon-mods/pull/857) | 6cff411a8b6b | 3cb745b58cfc | 3cb745b58cfc | 856 | yes | 6 |
| [#859](https://github.com/sofia-gros/horizon-mods/pull/859) | 126af15fe2de | 6cff411a8b6b | 6cff411a8b6b | 857 | yes | 6 |
| [#860](https://github.com/sofia-gros/horizon-mods/pull/860) | 298856c3e313 | 126af15fe2de | 126af15fe2de | 859 | yes | 5 |
| [#863](https://github.com/sofia-gros/horizon-mods/pull/863) | b68616ab77ad | e678dc43200e | e678dc43200e | 849 | yes | 6 |
| [#867](https://github.com/sofia-gros/horizon-mods/pull/867) | 9d2ba7fc0d27 | 298856c3e313 | 298856c3e313 | 860 | yes | 6 |
| [#869](https://github.com/sofia-gros/horizon-mods/pull/869) | f3498ffb4e0d | 9d2ba7fc0d27 | 9d2ba7fc0d27 | 867 | yes | 6 |
| [#870](https://github.com/sofia-gros/horizon-mods/pull/870) | df7efd895da8 | 177a77db5edc | 177a77db5edc | 838 | yes | 5 |
| [#872](https://github.com/sofia-gros/horizon-mods/pull/872) | 6d8e058038f4 | f3498ffb4e0d | f3498ffb4e0d | 869 | yes | 6 |
| [#900](https://github.com/sofia-gros/horizon-mods/pull/900) | 0f3aed6da7c1 | 6d8e058038f4 | 6d8e058038f4 | 872 | yes | 5 |
| [#918](https://github.com/sofia-gros/horizon-mods/pull/918) | 1636a304a157 | 0f3aed6da7c1 | 0f3aed6da7c1 | 900 | yes | 7 |
| [#922](https://github.com/sofia-gros/horizon-mods/pull/922) | 1ba0c453ebf8 | 1636a304a157 | 1636a304a157 | 918 | yes | 6 |

## Storage contract correspondence

67 PRs touch at least one source path used by the14-contract inventory. That is only a review index, not proof that an old head already uses Pluto or has equivalent behavior. The three other rows were manually reviewed:

- **#678:** Fixture/CI-only delta: validates the production Melter driver; no new reusable production operation in this patch. Incremental paths contain workflow, docs, test runner and test source only.

- **#704:** TConstruct-specific station part-swap repair/material/broken-tool policy; primary recipe/tool review, not another generic storage codec. Inspected full incremental Rust diff: finish_station_swap_repair, part_swap_repair_base and swap_plan_with_capacity preserve tic_max_damage/tic_broken and modifier calculation order.

- **#859:** Client/guest container commits preserve raw controller metadata, recheck it, then publish carried/inventory and cached views. Current quantity follow-up is mechanical and does not replace these commit rules. Inspected full source-smeltery-click/source-smeltery-guest-guard incremental diff; tank_to_nbt_preserving plus read-before-write gates.

Current foundation `89744814` remains separate from consumer follow-ups `375fa4ae` (slot quantity) and `b0e911b5` (fluid-slot quantity). Do not replace a whole legacy machine branch just because an overlapping codec/planner is now implemented. Preserve raw-field aliases, empty/zero-count policy, physical slot mappings and metadata/live/notification/held publication order.

Suggested provisional category handling: most rows belong to⑥ machines/storage with reusable mechanisms in① Pluto foundation. Casting/material/armor rows #696/#701/#703/#704/#710 require recipe/tool/armor owner review; #680/#705 also touch world-fluid content. These are consultation tags, not approved PR moves.

## Review order and exclusions

1. Start from the fixed main and verified shared foundation; mark the23 already-in-main heads as inherited history.
2. Follow fixed parent edges and inspect each actual-merge-base delta. Do not import all97 commits as a batch without reviewing merge and cumulative dependencies.
3. Use the65 shared-path groups to schedule combined review/tests, not to declare redundancy.
4. Pair mechanical shared-API adoption with its bounded differential evidence; keep complete original feature recovery and live authority gates separately open.

No retarget, merge, close, squash, force-push, PR edit, or new implementation was performed. The70-row scope does not cover all265 open PRs, all historical PRs, or the complete source universe.

## Reproduction

For each recorded MOD head/base, compute `git merge-base --all BASE HEAD` (exactly one was found), then `git diff --no-ext-diff --no-textconv --binary MERGE_BASE HEAD` and `git patch-id --stable`. Determine inherited fixed PR heads using `git rev-list HEAD`. Repeat independently against fixed main for the cumulative branch delta; keep direct two-tree main-to-head changes separate. Single-commit stable patch comparison skips merges rather than inventing a flattening policy. No upstream engine checkout is needed for these graph operations.
