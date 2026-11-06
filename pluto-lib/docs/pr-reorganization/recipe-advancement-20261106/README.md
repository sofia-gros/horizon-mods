# Recipe / advancement PR delta ledger

This is a **read-only regrouping aid**, not a final PR assignment or merge plan. No PR was closed, retargeted, merged or edited. No new production consumer, test, CI check, native plan or build was run for this ledger. Only existing `sofia-gros/horizon-mods` Git objects were read; no discarded Horizon fork was accessed.

## Frozen scope and evidence

- 51 PRs from the completed acquisition audit are selected. This is not an assertion that every recipe/station change in the repository belongs here.
- `frozen-prs.json` retains the coordinator's 265-PR snapshot, including exact head and base SHAs. Branch names are labels, **not** substitutes for those SHAs or a claim about today's branch tips.
- `scope.json` records the six provisional review subgroups, historical audit findings and the original snapshot hash.
- `ledger.json` records each actual merge-base, all head parents, patch SHA256/stable patch ID, changed paths, native manifest descriptor changes, declared stack ancestry and intersections.
- `file-deltas.csv` records every selected file transition: status, old/new mode and blob ID, line counts and exact per-file patch hash. Binary assets are included.
- `path-intersections.csv` separates selected PRs from external PRs touching the same file. Such intersections are review hot spots, **not proof of duplicate behavior or textual conflicts**.
- `migration-map.json` connects concrete old caller groups to the separately reviewed shared-boundary inventory. This is a migration responsibility map, not a receipt that old heads already use PlutoLib.

The shared-library comparison is fixed to local `8715b1232c0fca133fe012f35830e2108e653d59`, corresponding to remote [`89744814`](https://github.com/sofia-gros/horizon-mods/commit/89744814b8d536a117e82f8ea3c8cdc9798faa95), tree `267be30ceab0c59e8763a1e2de4db0f4fff83cf5`. The separate [28-boundary inventory](https://github.com/sofia-gros/horizon-mods/blob/69e3ddc6f220def799031813ed025b19842de1f9/pluto-lib/docs/recipe-advancement-boundary-inventory-20261106.md) remains the source-contract evidence, not an original-PR verification result.

## Actual diff result

- 51 / 51 selected head/base pairs resolved; all 51 bases are ancestors of their heads.
- 1,120 file-delta records across 1,012 distinct paths.
- 36 stacked PRs and 15 roots; the #675 → #941 chain has 33 PRs including its root.
- 56 shared-path review intersections, including 7 paths also changed outside this scope.
- No identical whole stable-patch-ID groups and no repeated exact same-path old→new blob/mode transition among the 51. This does **not** rule out semantic duplication or overlapping hunks.
- Five exact native target-string intersections with PlutoLib. Aliases and different entry points can create additional semantic intersections.
- 212 external PR diffs were available for path comparison. #944 and #945 remote objects were absent from this local object store; those two are explicitly excluded from the external diff sweep, not treated as empty or unchanged. Their fixed refs remain in the input. Guide content #693 → #805 belongs to its separate reviewer.

## Why whole trees or tip-only cherry-picks are unsafe

For each PR the authoritative delta here is `git diff merge-base(base_sha, head_sha) head_sha`, with both endpoints frozen. Since all selected bases are ancestors, this equals its base→head tree diff. It is **not** the tip commit alone and it is not the cumulative main→head diff.

- #941 contributes **14 changed files** relative to #938. Its cumulative comparison to historical main contains **478 paths**, of which **464 are outside that PR's own delta**. Taking its whole tree would silently import the advancement, station, variant and earlier fake-material work.
- #943 contributes **7 changed files** relative to #713; its main comparison has **13 paths**, six outside its own delta. Travelers and fake-material policies must remain separate even when they adopt the same index/preview capabilities.
- #665 and #675 are merge tips whose first parent is not the frozen PR base. A blind single-parent cherry-pick is not equivalent to either PR delta. All other 49 selected heads have their frozen base as first parent.
- A same-path update later in one stack is usually an extension or revision, not an independent feature copy. Example: #839 → #843 → #845 → #847 → #850 evolve one tool predicate; #855 adds the tank predicate while inheriting that history. Do not copy every full file version or drop the earlier criteria.

Historical main is `08afa992d5091e03360f3473ea93fd17aa793987`; each row records its actual merge-base with that main before counting cumulative paths. These counts describe paths, not independently implemented tasks.

## Native boundary correspondence and required sequencing

| Old PR delta / caller | Shared boundary | Reorganization constraint |
| --- | --- | --- |
| #675 tag index and `World::in_tag`; later allowlist/criteria updates | S04 tags/cache; S17 resources/tag graph | Port ready snapshot publication and clear/invalidation first. Remove competing direct index replacement only with consumer adoption. TConstruct tag scope/criteria stay local. |
| #839–#850 tool predicates; #855 tank predicate | S01 metadata/events; S02 predicate callbacks | Preserve each criterion, startup opt-in and canonical library identity. Library presence alone does not restore missing `ItemView.custom_data`. |
| #828/#831 successful controller-open events | S03 synchronous event sink | Preserve local/authoritative guest success gates and Smeltery/Foundry type policy; submission is not proof of awarding. Coordinate machine-open owner. |
| #861 station ingredient parsing | S11 item-ID ingredient dispatch | Port only the supported item-ID contract; do not pretend component-sensitive smithing is implemented. |
| #930 fake-material assembly HEAD | S10 deny-only match gate; B01 automatic authority remains open | This is an exact protected target collision. A veto cannot replace output-producing policy, and the staged pair API is not automatic consumption authority. Do not weaken the guard or simply delete output behavior. |
| #935 placement RETURN and book-click HEAD | S08 explicit preview; S09 deny-only placement | Both intersect Pluto-owned targets. Register consumer policy and preserve component-sensitive preview-only behavior before retiring hooks; preview never authorizes allocation. |
| #935/#943 index/UI helpers | S06 invalidation; S07 current-record CAS | Keep family-specific ID allocation and expected-record checks. No persistent ownership claim (B02); caller stages all related tables before publication. |
| #938 prepared-world book handoff | S13 scoped saved knowledge handoff | Preserve exact world readiness and accepted IDs; not a generic global cache. |
| #941 dynamic discovery tree | S05 loaded-tree projection | Preserve IDs, criteria, rewards and native progress. No unsupported `AdvState` field; guest publication/lifecycle remains B03. |
| #713 Travelers crafting/crafter route | Existing shared read-only/match services plus B01 | Its obsolete exact expression selector and separate crafter authority need actual recovery. Recipe-book extraction does not fix crafting or map/ejection ordering. |

The machine-readable manifest intersections are exact strings. Two more old item predicate targets use `horizon_client::advancements::item`, and #861 uses `horizon_client::stations::ingredient`; the historical audit flags their canonical library identity problem. They are not silently normalized into successful target resolution here.

Pluto's frozen composition policies are visible in [advancement replacements](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/tools/advancement-replacements.json), [crafting match](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/tools/crafting-match-boundaries.json), and [recipe placement](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/tools/recipe-placement-boundaries.json). Exact-string intersection alone is not a newly executed composition check; `in_tag` is not covered by the full-body replacement exclusive-target list.

## Cross-category hot spots

The seven external shared paths are the TConstruct manifest, `recipe-reload.rs`, `source-station-world.rs`, `source-melter-world.rs`, `source-machine-open.rs`, `tests/condition-snapshot/src/lib.rs`, and `.github/workflows/mixins.yml`. Their complete PR sets are in the CSV/JSON. Broad file copying or choosing one branch's version risks losing other groups' module registrations, reload calls, world state or tests.

#717 mixes colored slime block assets/registration with acquisition recipes. #781/#858–#926 contain station recipe/variant data and generators, not just advancement logic. #828 modifies machine opening paths. Keep these cross-category responsibilities explicit until the parent/user approves the final broad grouping. #693/#805 guide content and #944/#945 shared foundation/navigation are references to coordinate, not reassigned here.

## Per-PR delta inventory

`Own / cumulative` means changed path counts, not functionality coverage. Full immutable refs, per-file blobs and hashes are in the JSON/CSV. Group names are review aids only.

| PR | Review subgroup | Frozen base | Frozen head | Parent PR | Own / cumulative paths |
| --- | --- | --- | --- | --- | --- |
| [#665](https://github.com/sofia-gros/horizon-mods/pull/665) | configured_crafting | `3dc3d785` | `476a05fb` | root | 10 / 10 |
| [#675](https://github.com/sofia-gros/horizon-mods/pull/675) | advancement_acquisition | `af47e668` | `c4c250f3` | root | 10 / 10 |
| [#713](https://github.com/sofia-gros/horizon-mods/pull/713) | travelers_material_recipe_book | `6d980fa3` | `bbfeb25e` | root | 10 / 10 |
| [#717](https://github.com/sofia-gros/horizon-mods/pull/717) | static_unlocks_and_slime_content | `6d980fa3` | `632463ea` | root | 32 / 32 |
| [#721](https://github.com/sofia-gros/horizon-mods/pull/721) | static_unlocks_and_slime_content | `6d980fa3` | `730ffff9` | root | 28 / 28 |
| [#727](https://github.com/sofia-gros/horizon-mods/pull/727) | static_unlocks_and_slime_content | `6d980fa3` | `a9ab1505` | root | 84 / 84 |
| [#729](https://github.com/sofia-gros/horizon-mods/pull/729) | static_unlocks_and_slime_content | `6d980fa3` | `d3c0b0ad` | root | 102 / 102 |
| [#735](https://github.com/sofia-gros/horizon-mods/pull/735) | static_unlocks_and_slime_content | `6d980fa3` | `b3d99db7` | root | 44 / 44 |
| [#739](https://github.com/sofia-gros/horizon-mods/pull/739) | static_unlocks_and_slime_content | `6d980fa3` | `baf015db` | root | 28 / 28 |
| [#741](https://github.com/sofia-gros/horizon-mods/pull/741) | static_unlocks_and_slime_content | `6d980fa3` | `11fbe946` | root | 16 / 16 |
| [#744](https://github.com/sofia-gros/horizon-mods/pull/744) | static_unlocks_and_slime_content | `6d980fa3` | `0a9ee50f` | root | 112 / 112 |
| [#748](https://github.com/sofia-gros/horizon-mods/pull/748) | static_unlocks_and_slime_content | `6d980fa3` | `73fa7400` | root | 16 / 16 |
| [#751](https://github.com/sofia-gros/horizon-mods/pull/751) | static_unlocks_and_slime_content | `6d980fa3` | `f39c02ae` | root | 12 / 12 |
| [#754](https://github.com/sofia-gros/horizon-mods/pull/754) | static_unlocks_and_slime_content | `6d980fa3` | `2a536a92` | root | 10 / 10 |
| [#760](https://github.com/sofia-gros/horizon-mods/pull/760) | advancement_acquisition | `c4c250f3` | `c592bc10` | #675 | 38 / 42 |
| [#763](https://github.com/sofia-gros/horizon-mods/pull/763) | advancement_acquisition | `c592bc10` | `e3503a89` | #760 | 12 / 50 |
| [#769](https://github.com/sofia-gros/horizon-mods/pull/769) | advancement_acquisition | `e3503a89` | `a1c977c2` | #763 | 12 / 58 |
| [#771](https://github.com/sofia-gros/horizon-mods/pull/771) | static_unlocks_and_slime_content | `632463ea` | `5e1b6a65` | #717 | 10 / 41 |
| [#775](https://github.com/sofia-gros/horizon-mods/pull/775) | advancement_acquisition | `a1c977c2` | `6ca0bfb7` | #769 | 6 / 64 |
| [#778](https://github.com/sofia-gros/horizon-mods/pull/778) | advancement_acquisition | `6ca0bfb7` | `c7522dec` | #775 | 10 / 74 |
| [#781](https://github.com/sofia-gros/horizon-mods/pull/781) | station_variants_and_live_tags | `c7522dec` | `284a3a27` | #778 | 25 / 95 |
| [#786](https://github.com/sofia-gros/horizon-mods/pull/786) | configured_crafting | `6d980fa3` | `01442ca3` | root | 7 / 7 |
| [#790](https://github.com/sofia-gros/horizon-mods/pull/790) | advancement_acquisition | `284a3a27` | `2d812b4c` | #781 | 13 / 105 |
| [#809](https://github.com/sofia-gros/horizon-mods/pull/809) | static_unlocks_and_slime_content | `5e1b6a65` | `ce616021` | #771 | 10 / 50 |
| [#814](https://github.com/sofia-gros/horizon-mods/pull/814) | advancement_acquisition | `2d812b4c` | `044cd37c` | #790 | 4 / 105 |
| [#815](https://github.com/sofia-gros/horizon-mods/pull/815) | configured_crafting | `476a05fb` | `01f917a0` | #665 | 9 / 15 |
| [#818](https://github.com/sofia-gros/horizon-mods/pull/818) | advancement_acquisition | `044cd37c` | `64ea6719` | #814 | 12 / 114 |
| [#821](https://github.com/sofia-gros/horizon-mods/pull/821) | advancement_acquisition | `64ea6719` | `bcc3da97` | #818 | 6 / 120 |
| [#823](https://github.com/sofia-gros/horizon-mods/pull/823) | advancement_acquisition | `bcc3da97` | `0de88796` | #821 | 11 / 128 |
| [#828](https://github.com/sofia-gros/horizon-mods/pull/828) | advancement_acquisition | `0de88796` | `9ea252b1` | #823 | 15 / 141 |
| [#831](https://github.com/sofia-gros/horizon-mods/pull/831) | advancement_acquisition | `9ea252b1` | `cd9b36bb` | #828 | 10 / 147 |
| [#839](https://github.com/sofia-gros/horizon-mods/pull/839) | advancement_acquisition | `cd9b36bb` | `36406a32` | #831 | 15 / 158 |
| [#843](https://github.com/sofia-gros/horizon-mods/pull/843) | advancement_acquisition | `36406a32` | `32b2c909` | #839 | 13 / 168 |
| [#845](https://github.com/sofia-gros/horizon-mods/pull/845) | advancement_acquisition | `32b2c909` | `9c1265cf` | #843 | 11 / 175 |
| [#847](https://github.com/sofia-gros/horizon-mods/pull/847) | advancement_acquisition | `9c1265cf` | `491b986a` | #845 | 16 / 183 |
| [#850](https://github.com/sofia-gros/horizon-mods/pull/850) | advancement_acquisition | `491b986a` | `7840b3cf` | #847 | 14 / 193 |
| [#855](https://github.com/sofia-gros/horizon-mods/pull/855) | advancement_acquisition | `7840b3cf` | `7954d235` | #850 | 11 / 202 |
| [#858](https://github.com/sofia-gros/horizon-mods/pull/858) | station_variants_and_live_tags | `7954d235` | `38a17194` | #855 | 60 / 262 |
| [#861](https://github.com/sofia-gros/horizon-mods/pull/861) | station_variants_and_live_tags | `38a17194` | `155ebdb8` | #858 | 35 / 293 |
| [#864](https://github.com/sofia-gros/horizon-mods/pull/864) | station_variants_and_live_tags | `155ebdb8` | `72e04e6b` | #861 | 10 / 303 |
| [#868](https://github.com/sofia-gros/horizon-mods/pull/868) | station_variants_and_live_tags | `72e04e6b` | `a5e19bac` | #864 | 14 / 316 |
| [#914](https://github.com/sofia-gros/horizon-mods/pull/914) | station_variants_and_live_tags | `a5e19bac` | `548a004f` | #868 | 49 / 358 |
| [#919](https://github.com/sofia-gros/horizon-mods/pull/919) | station_variants_and_live_tags | `548a004f` | `4f45b085` | #914 | 22 / 378 |
| [#921](https://github.com/sofia-gros/horizon-mods/pull/921) | station_variants_and_live_tags | `4f45b085` | `42582c9d` | #919 | 37 / 414 |
| [#924](https://github.com/sofia-gros/horizon-mods/pull/924) | station_variants_and_live_tags | `42582c9d` | `42738ece` | #921 | 29 / 440 |
| [#926](https://github.com/sofia-gros/horizon-mods/pull/926) | station_variants_and_live_tags | `42738ece` | `788b318c` | #924 | 10 / 446 |
| [#930](https://github.com/sofia-gros/horizon-mods/pull/930) | fake_material_recipe_book | `788b318c` | `56c75bee` | #926 | 12 / 456 |
| [#935](https://github.com/sofia-gros/horizon-mods/pull/935) | fake_material_recipe_book | `56c75bee` | `f7cd7c23` | #930 | 10 / 464 |
| [#938](https://github.com/sofia-gros/horizon-mods/pull/938) | fake_material_recipe_book | `f7cd7c23` | `1a5f91b2` | #935 | 7 / 468 |
| [#941](https://github.com/sofia-gros/horizon-mods/pull/941) | fake_material_recipe_book | `1a5f91b2` | `8c56bb52` | #938 | 14 / 478 |
| [#943](https://github.com/sofia-gros/horizon-mods/pull/943) | travelers_material_recipe_book | `bbfeb25e` | `a8fc1e18` | #713 | 7 / 13 |

## Reproduce and check

Run `python reproduce.py --repo /path/to/horizon-mods --out /tmp/recipe-pr-ledger` using an existing MOD object store containing the frozen commits. It reads no remote, runs no Cargo/tests and does not edit Git refs or PR state. Missing selected commits fail the run; missing external commits are reported. Compare `ledger.json`, `file-deltas.csv` and `path-intersections.csv` byte-for-byte with this directory. `frozen-prs.json` and `scope.json` are hashed inside the ledger.

Validation for this documentation change is a second independent regeneration plus count/ref/CSV consistency checks, not source-runtime verification. Historical `needs_fix`/`unverified` findings in each row have not been upgraded by shared-library proofs or this inventory. No files or hooks are declared safe to delete automatically. Review and test the eventual coherent category branch after resolving the listed ownership and composition boundaries.
