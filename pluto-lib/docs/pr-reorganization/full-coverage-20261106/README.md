# Fixed 265-PR coverage and safe intake plan

This is a read-only inventory checkpoint, not an intake branch or a claim that 265 PRs are migrated, reviewed, tested, or ready to merge. No existing PR was closed, retargeted, edited, approved, or merged. No runtime source or workflow is changed by this checkpoint.

## What is complete

The immutable 2026-11-06 snapshot contains 265 open PRs. Every PR now has exactly one primary research row, an exact published head, the REST-reported base, an effective own-delta base, a diff digest, and a link to its detailed domain ledger. All 265 head/base pairs match the fixed snapshot; missing rows and duplicate primary rows are both zero.

| Research partition | Primary rows | Detailed immutable ledger |
| --- | ---: | --- |
| Recipe and advancement | 51 | [ab037b6a](https://github.com/sofia-gros/horizon-mods/commit/ab037b6a708b647c9ed90af33a661adca49df1c7) |
| Storage and machines | 70 | [5c4e7562](https://github.com/sofia-gros/horizon-mods/commit/5c4e75628259ee10cf4f7943b1fa6632ffe44f54) |
| Guide | 24 | [4294fb98](https://github.com/sofia-gros/horizon-mods/commit/4294fb986c6cadca9ad66c73c5d781a2cf039a5d) |
| Foundation #944 | 1 | Same Guide ledger, counted only here |
| World and fluids | 23 | [5f6c2053 contracts](https://github.com/sofia-gros/horizon-mods/commit/5f6c20539d7d7740bf559d06de60b52f38f62053), actual-base supplement in [102aa57d](https://github.com/sofia-gros/horizon-mods/commit/102aa57d27a88542cea4fae8ec0b472043ec7d57) |
| Armor, defense and movement | 59 | [102aa57d](https://github.com/sofia-gros/horizon-mods/commit/102aa57d27a88542cea4fae8ec0b472043ec7d57) |
| Remaining items, tools and combat | 37 | [04c66027](https://github.com/sofia-gros/horizon-mods/commit/04c66027876fb4ebbce2833acdf872a090259ec4) |

These seven research partitions are not the proposed eight feature/review PRs. Cross-category contracts remain linked in the detailed ledgers rather than duplicating PR counts. The eight proposed product groups remain: shared foundation; items/material/data; recipes/stations/advancement; tools/combat; armor/defense/movement; machines/storage; world-fluid/rendering; guide/UI/LAN. They are a review plan, not permission to replace or close old PRs.

## Fixed comparison basis

- MOD repository: `sofia-gros/horizon-mods` only.
- Main: `08afa992d5091e03360f3473ea93fd17aa793987`.
- Central local v15: `8715b1232c0fca133fe012f35830e2108e653d59`.
- Exact central remote snapshot: `89744814b8d536a117e82f8ea3c8cdc9798faa95`.
- Original snapshot and its read time are pinned by SHA-256 in `coverage.json`. This report does not pretend that the fixed snapshot is a new live PR query.
- Each domain input is an immutable published document with its own SHA-256. The aggregator reads these documents, not arbitrary branch tips.

The intended dependency structure is **Horizon → Mixins → PlutoLib for opting-in mods, otherwise directly → Mixins Mod**. Tinkers uses PlutoLib. Shared behavior belongs in PlutoLib where its contract is genuinely reusable; Tinkers keeps its content, schemas, formulas, eligibility, layout, and actor-specific policy. This does not make PlutoLib mandatory for every Mixins mod.

## Important intake hazards

1. **REST base is not always the effective own-delta base.** The reported base is not an ancestor for #676, #678, #679, #693, #694, #695, #696, #701, #702, and #704. Preserve that distinction. For #693, endpoint base-to-head comparison includes 46 files and 621 deletions, whereas the actual own delta has six files. Replaying the former would import base-side reversals.
2. **Do not replay a stacked head as a single feature.** #941 has 14 own paths versus 478 cumulative paths; #936 has six versus 29. Guide #945 has 11 rename-aware own entries and inherits the 476-entry #944 foundation, not the separate 23-PR guide content chain. #944 itself has 444 exact-central file states and 32 evolved ones, so a wholesale replay can roll back newer work.
3. **Merge tips need parent-aware intake.** The detailed ledgers identify examples including #665, #675, #683, and #691. A merge commit is not automatically its first-parent feature delta. Preserve prerequisite and provenance information instead of blindly cherry-picking it.
4. **Same paths are not duplicates.** Storage has 51 PRs touching `machine-runtime.rs`. Other ledgers record shared paths and exact native-hook targets. They are review hotspots, not authority to remove one behavior. No global semantic-duplicate claim is made by this aggregation.
5. **Ancestry is not acceptance.** Twenty-three storage heads already occur in fixed main history, but this does not authorize closing them or establish current feature parity. Similarly, existing or inherited Pluto references do not prove a consumer migration. The armor audit found no newly added defense-facade integration in its scoped diffs.
6. **Keep incompatible proof scopes separate.** v15 is the ordinary shared-library plan/apply/type checkpoint. It does not validate every old consumer. Independent slot/fluid/Java/menu/tank fixtures are separate cutoffs. Guide response lifetime, automatic paired crafting, effect ownership/persistence/contact, and actual rendering remain distinct unresolved contracts.

Path counts preserve each auditor's convention. Guide entries are rename-aware, while the other ledgers retain no-rename path endpoints. This report deliberately does not sum these counts into a purported unique-file or work-completion metric. The #944/#945 local object gap is covered by the Guide ledger's recorded remote ancestry and exact-tree evidence, not concealed as a successful local Git lookup.

## Step 2 priority and safe consolidation

1. Preserve all original PR heads and published checkpoints. Use the fixed ledgers as an intake manifest, with explicit retained features, inherited prerequisites, and native hook ownership for each selected own delta.
2. Stabilize the broad Pluto foundation first. Source integration of Java ordering and menu framing is already saved at [7aaf04c6](https://github.com/sofia-gros/horizon-mods/commit/7aaf04c6091b74a6d343137a26aa95069c9b7e7a); its source/package checks passed, but its new combined Rust/API test stage has not started. Earlier individual focused proofs do not stand in for that combined check.
3. Bring in consumer migrations one coherent group at a time, keeping the old behavior as a frozen oracle where practical. Examples already separately preserved include [slot insertion](https://github.com/sofia-gros/horizon-mods/commit/375fa4aef1e70f17e5d79d2e5bbf94bbcb87192f), [fluid-slot quantity](https://github.com/sofia-gros/horizon-mods/commit/b0e911b5a294c430a471e168f86c7ca0ed7152f4), and [tank compound preservation](https://github.com/sofia-gros/horizon-mods/commit/feeedb14a863c48cc1940e7014768cd3af4bbdda). Their focused passes are not full Tinkers, world-save, or LAN acceptance.
4. Reconcile schema, manifest, runtime-helper and workflow overlaps before running the exact combined package. Require exact-source seals, package guards, actual changed-crate compiler evidence, and recorded dependency identity. A matching Cargo path/lock is insufficient to guarantee cache reuse: the tank fixture rebuilt its API due to a different dependency fingerprint.
5. Keep Actions-dependent native Game/renderer checks pending while the user cannot operate Actions. The manual [9385b77a checkpoint](https://github.com/sofia-gros/horizon-mods/commit/9385b77a37f19fab517b5a1ec09b4e1f8c824408) is prepared, not executed. Do not replace it with unsafe local compilation under an unobservable RAM limit.
6. Only after feature mapping, scoped migration and exact combined tests should the proposed 5–10 larger PRs be formed for review. Original PR closure, retargeting or main integration is a separate action, not performed here. No reliable main-merge date follows from inventory coverage alone.

Small, explicit source checkpoints continue while capacity is constrained. Native builds and large writes remain gated. Preserve at least the user's 10 GiB requirement; the usual native guard is 10 GiB + 128 MiB. Any separately authorized lower recovery margin applies only to preservation writes, not to a build. Unknown-provenance files, host dependency caches, source, logs, and unpublished work stay protected.

## Reproduction and verification

`build_coverage.py` accepts `--snapshot`, `--recipe`, `--storage`, `--guide`, `--armor-world`, `--items-tools`, and `--out`. Supply the exact documents from the source URLs in `coverage.json` and the frozen snapshot in the preceding audit checkpoint. It verifies all six input hashes before normalizing rows, then asserts exactly 265 unique primary IDs, equality to the snapshot set, all head/base pairs, all own-delta bases and diff digests, and the seven expected partition counts.

The script only reads the supplied JSON files and writes the chosen output file. It does not fetch repositories, read engine forks, invoke Git/Cargo, alter PRs, or execute native code. Repeating it with the same immutable inputs produces byte-identical JSON. Independent domain review and aggregate verification are recorded in `validation.json`.
