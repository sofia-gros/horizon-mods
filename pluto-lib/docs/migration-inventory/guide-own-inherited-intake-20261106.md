# Guide PR intake: own changes and inherited changes

This is a read-only source intake map, not an integration or merge result.
No PR state/metadata, production source, native stage, CI job or guard changed.
No new build or test was run. The 25 PR records are separate from the earlier
8 contract-boundary records; category08 owns 24 PRs, with #944 cross-linked from
category01.

## Immutable inputs

- Main: `08afa992d5091e03360f3473ea93fd17aa793987`.
- Central source: local `8715b123`, remote `89744814`, tree `267be30c`.
- [Coordinator snapshot](https://github.com/sofia-gros/horizon-mods/blob/4bf5fe7d06a08ae72330dea44ddfc445471f073a/pluto-lib/docs/pr-reorganization-snapshot-20261106.json): 265 fixed PR records, rechecked at 04:39:33 UTC.
- Official target: Horizon `624c4f62` and Mixins `e8cd0aa3`.

The JSON records full fixed head, reported base, actual own-delta base,
source trees, before/after blob IDs, statuses, inherited PRs, patch hashes,
central comparison and proposed intake route. Stored REST base SHA is not
assumed to be a merge-base. Renames are counted as one entry.

## Three findings that prevent accidental loss

1. **#693 requires its actual common ancestor.** Its reported base `10ad8a4f`
   is not an ancestor of head `38a804e4`. The actual merge-base is `442946c5`.
   The intended own delta is 6 files, 209 insertions and 4 deletions. A raw
   reported-base-to-head diff is 46 files, 245 insertions and 621 deletions because
   it also reverses 40 later base-side files. Do not use that endpoint diff for
   intake. All other 22 content-chain bases are the exact preceding PR heads.
2. **The 23 content PRs are one stack, not 23 independent snapshots.** Their
   incremental changes touch 191 file entries; the unique/cumulative tip delta
   has 63 paths (43 added, 20 modified). Most later PRs revisit inherited paths.
   Consume each own delta once, or trace every contribution into one reviewed
   cumulative tip. Do not reapply complete head trees or sum repeated paths.
3. **#945 inherits foundation, not the 23 content PRs.** Its 11 own rename-aware
   entries sit on #944's 476-file v13 snapshot. #944 has 444 changed-path blobs
   already identical to current central and 32 evolved paths. Replacing central
   with either whole snapshot would lose later guide Text, manifest, guard,
   inventory/crafting/render and proof updates. Take the navigation delta only;
   preserve the newer category01 foundation.

## Fixed per-PR source map

Short SHAs are displayed below; complete identities are in the JSON. Own paths
include authored tests/docs and are not counts of completed features. Inherited
counts refer to other PR deltas, not new changes authored by that row.

| PR | Fixed head | Reported base | Effective own base | Own entries | Inherited PRs | Intake |
|---|---|---|---|---:|---:|---|
| [#693](https://github.com/sofia-gros/horizon-mods/pull/693) | `38a804e4` | `10ad8a4f` | `442946c5` | 6 | 0 | guide chain own delta |
| [#698](https://github.com/sofia-gros/horizon-mods/pull/698) | `a17b0566` | `38a804e4` | `38a804e4` | 12 | 1 | guide chain own delta |
| [#707](https://github.com/sofia-gros/horizon-mods/pull/707) | `c20813dc` | `a17b0566` | `a17b0566` | 8 | 2 | guide chain own delta |
| [#708](https://github.com/sofia-gros/horizon-mods/pull/708) | `d5a5eeaa` | `c20813dc` | `c20813dc` | 7 | 3 | guide chain own delta |
| [#709](https://github.com/sofia-gros/horizon-mods/pull/709) | `416850d4` | `d5a5eeaa` | `d5a5eeaa` | 8 | 4 | guide chain own delta |
| [#711](https://github.com/sofia-gros/horizon-mods/pull/711) | `22f34b2b` | `416850d4` | `416850d4` | 7 | 5 | guide chain own delta |
| [#712](https://github.com/sofia-gros/horizon-mods/pull/712) | `33e2f40b` | `22f34b2b` | `22f34b2b` | 8 | 6 | guide chain own delta |
| [#718](https://github.com/sofia-gros/horizon-mods/pull/718) | `5e9d740e` | `33e2f40b` | `33e2f40b` | 6 | 7 | guide chain own delta |
| [#725](https://github.com/sofia-gros/horizon-mods/pull/725) | `ff81d20a` | `5e9d740e` | `5e9d740e` | 8 | 8 | guide chain own delta |
| [#732](https://github.com/sofia-gros/horizon-mods/pull/732) | `9567553e` | `ff81d20a` | `ff81d20a` | 14 | 9 | guide chain own delta |
| [#737](https://github.com/sofia-gros/horizon-mods/pull/737) | `486995af` | `9567553e` | `9567553e` | 8 | 10 | guide chain own delta |
| [#746](https://github.com/sofia-gros/horizon-mods/pull/746) | `132521b8` | `486995af` | `486995af` | 13 | 11 | guide chain own delta |
| [#753](https://github.com/sofia-gros/horizon-mods/pull/753) | `74e4ced3` | `132521b8` | `132521b8` | 6 | 12 | guide chain own delta |
| [#758](https://github.com/sofia-gros/horizon-mods/pull/758) | `647173b1` | `74e4ced3` | `74e4ced3` | 6 | 13 | guide chain own delta |
| [#767](https://github.com/sofia-gros/horizon-mods/pull/767) | `1b605578` | `647173b1` | `647173b1` | 12 | 14 | guide chain own delta |
| [#774](https://github.com/sofia-gros/horizon-mods/pull/774) | `134a84f6` | `1b605578` | `1b605578` | 6 | 15 | guide chain own delta |
| [#776](https://github.com/sofia-gros/horizon-mods/pull/776) | `1031e51e` | `134a84f6` | `134a84f6` | 5 | 16 | guide chain own delta |
| [#782](https://github.com/sofia-gros/horizon-mods/pull/782) | `22abb2f0` | `1031e51e` | `1031e51e` | 8 | 17 | guide chain own delta |
| [#787](https://github.com/sofia-gros/horizon-mods/pull/787) | `d213d180` | `22abb2f0` | `22abb2f0` | 7 | 18 | guide chain own delta |
| [#797](https://github.com/sofia-gros/horizon-mods/pull/797) | `1103b360` | `d213d180` | `d213d180` | 5 | 19 | guide chain own delta |
| [#800](https://github.com/sofia-gros/horizon-mods/pull/800) | `75bcec52` | `1103b360` | `1103b360` | 6 | 20 | guide chain own delta |
| [#803](https://github.com/sofia-gros/horizon-mods/pull/803) | `cdf63c12` | `75bcec52` | `75bcec52` | 13 | 21 | guide chain own delta |
| [#805](https://github.com/sofia-gros/horizon-mods/pull/805) | `0e334211` | `cdf63c12` | `cdf63c12` | 12 | 22 | guide chain own delta |
| [#944](https://github.com/sofia-gros/horizon-mods/pull/944) | `bcec17b5` | `08afa992` | `08afa992` | 476 | 0 | category01 dependency |
| [#945](https://github.com/sofia-gros/horizon-mods/pull/945) | `d4d798fb` | `bcec17b5` | `bcec17b5` | 11 | 1 | navigation delta only |

## Proposed intake responsibilities

- **Chapter and guide content:** retain source text, assets, IDs, notices and
  bounded/unfinished-feature wording in TConstruct. Generic UI code must not
  absorb these pages wholesale.
- **Eligibility policy:** #693/#698/#707/#708/#709/#711/#712/#718/#725 also
  change modifier, upgrade or tag helpers. Coordinate those policies with the
  modifier/recipe group; they are not documentation-only changes.
- **Guide presentation:** keep source layout, two-page spreads, title/root
  metadata, cover/history, visual selection and recipe display choices local.
  Convert reusable font/Text/resource mechanics through reviewed Pluto APIs,
  preserving the original accepted subset and work limits.
- **#945 navigation:** retain key/click/control routing and native ordinary,
  Edit and lectern behavior. The old response hook still blocks full guide
  composition; do not disable LAN refresh or weaken the guard to get a pass.
- **Foundation:** #944 belongs to category01. Its full immutable diff is
  identified by trees and patch hash. The JSON lists all 32 evolved own paths
  requiring preservation review rather than redundantly listing 444 unchanged
  blobs. The newer local central history contains the v13 runtime ancestor;
  remote snapshot ancestry is not substituted for that evidence.

Per-file roles are triage, not per-hunk behavior proof. For example #753's wire
module delta adds tests inside the production Rust file. A file classified as
a guide consumer does not imply every changed line executes at runtime.

## Authored, integrated and verified are different states

The original content changes are authored and published at fixed heads. This
map does not apply them to current main or claim semantic equivalence from
blob comparisons. All 23 original content heads still lack established complete
latest-official consumer/game/LAN proof.

#944 has its own v13 ordinary-library plan/guards/apply/type proof, not a full
consumer proof. #945 has 27 source/guard cases, 8 modeled Game/audio helper cases,
and an actual navigation-only overlay plan with 11 guards. Its native apply/type,
real input/audio/window and LAN proof remain absent. The interrupted full
consumer plan is inconclusive.

The storage/lifetime proposals remain excluded and unapplied. Actual Game
metadata/capacity/type gates and real retained-view/late-response proof remain
pending; current CI design prioritizes World's separate borrow-only probes.
No test count is added by this source inventory.

## Reproducing the object audit

Use only an authorized horizon-mods checkout with the listed existing objects,
the exact published coordinator snapshot, and a separate report output path:

    python3 audit-guide-intake.py --repo APPROVED_HORIZON_MODS_CHECKOUT \
      --snapshot pr-reorganization-snapshot-20261106.json \
      --output guide-own-inherited-intake-20261106.json

The auditor does not fetch, apply patches, run Cargo or change PRs. Remote
#944/#945 commit-parent chains were read independently; their exact tree
objects already existed locally and are used for byte-level diffing. The script
fails if the pinned coordinator snapshot changes. Immutable trees/blobs define
the source identity; patch hashes describe the observed Git diff serialization.

Conditional retirement still requires an explicitly equivalent grouped
replacement, exact combined official gates and affected gameplay/LAN proof,
followed by a separately authorized PR-state decision. **Do not close, retarget,
mark ready, approve or merge existing PRs based on this map.**
