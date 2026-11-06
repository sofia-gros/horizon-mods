# Guide / UI / LAN migration inventory

Read-only inventory dated 2026-11-06. No existing PR state or metadata changed,
no CI result inspected, no merge, and no new native check or test was run.
Only horizon-mods and pinned official sources were used.

## Cutoff and counts

- Main: `08afa992d5091e03360f3473ea93fd17aa793987`, verified from the branch ref.
- Shared source: local `8715b123`, remote [89744814](https://github.com/sofia-gros/horizon-mods/commit/89744814b8d536a117e82f8ea3c8cdc9798faa95), tree `267be30c`.
- Official Horizon `624c4f62`, Mixins `e8cd0aa3`.
- **25 PR rows**: 23 guide content-chain PRs, #945 navigation repair, and #944
  shared-foundation dependency. All were freshly confirmed open draft.
- **8 separate contract-boundary rows**. These are not eight additional PRs.
- Primary category08 has24PR rows; #944 is cross-linked from category01.

The adjacent JSON uses the coordinator's exact265-row PR dependency snapshot.
A PR's stored base SHA can be stale: #693's REST base snapshot differs from the
separately verified main ref. The ledger retains both without conflating them.

## Existing content chain

- [#693](https://github.com/sofia-gros/horizon-mods/pull/693) Show supported bonus modifier recipes in Puny Smelting (`38a804e4`).
- [#698](https://github.com/sofia-gros/horizon-mods/pull/698) Show ordinary modifier recipes in Puny and Mighty guides (`a17b0566`).
- [#707](https://github.com/sofia-gros/horizon-mods/pull/707) Show supported incremental modifier recipe guides (`c20813dc`).
- [#708](https://github.com/sofia-gros/horizon-mods/pull/708) Show bulk modifier inputs and leftover outputs in guides (`d5a5eeaa`).
- [#709](https://github.com/sofia-gros/horizon-mods/pull/709) Show repeatable fluid modifier recipes and empty containers (`416850d4`).
- [#711](https://github.com/sofia-gros/horizon-mods/pull/711) Show multi-input bonus recipes and bottle returns in Puny (`22f34b2b`).
- [#712](https://github.com/sofia-gros/horizon-mods/pull/712) Show Step Up, Necrotic and Gilded level-range guides (`33e2f40b`).
- [#718](https://github.com/sofia-gros/horizon-mods/pull/718) Show modifier-crystal alternatives in Puny and Mighty guides (`5e9d740e`).
- [#725](https://github.com/sofia-gros/horizon-mods/pull/725) Show slotless Blindshot recipe in Puny Smelting (`ff81d20a`).
- [#732](https://github.com/sofia-gros/horizon-mods/pull/732) Show seven broad-tool pages in Mighty Smelting (`9567553e`).
- [#737](https://github.com/sofia-gros/horizon-mods/pull/737) Explain supported tool tanks and capacity loss in Mighty (`486995af`).
- [#746](https://github.com/sofia-gros/horizon-mods/pull/746) Restore Mighty ability overviews and source recipe groups (`132521b8`).
- [#753](https://github.com/sofia-gros/horizon-mods/pull/753) Display five separate modifier recipe inputs in guides (`74e4ced3`).
- [#758](https://github.com/sofia-gros/horizon-mods/pull/758) Show incremental modifier progress and conditional returns (`647173b1`).
- [#767](https://github.com/sofia-gros/horizon-mods/pull/767) Show a bounded two-layer Smeltery construction plan (`1b605578`).
- [#774](https://github.com/sofia-gros/horizon-mods/pull/774) Keep material casting buckets paired with fluid tooltips (`134a84f6`).
- [#776](https://github.com/sofia-gros/horizon-mods/pull/776) Show all bounded composite material fluid alternatives (`1031e51e`).
- [#782](https://github.com/sofia-gros/horizon-mods/pull/782) Group Puny upgrade recipes under source categories (`22abb2f0`).
- [#787](https://github.com/sofia-gros/horizon-mods/pull/787) Keep Puny slotless recipes with their source categories (`d213d180`).
- [#797](https://github.com/sofia-gros/horizon-mods/pull/797) Gate static book crafting previews on live recipe support (`1103b360`).
- [#800](https://github.com/sofia-gros/horizon-mods/pull/800) Explain Crafting Station recipes with bounded difference inputs (`75bcec52`).
- [#803](https://github.com/sofia-gros/horizon-mods/pull/803) Show Mighty defense overviews with bounded recipe guidance (`cdf63c12`).
- [#805](https://github.com/sofia-gros/horizon-mods/pull/805) Show qualified Mighty entity-melting reference chapter (`0e334211`).

These original exact heads are unchanged from the earlier source audit and are
not ancestors of the current main cutoff. None of their inventoried changed
blobs exactly matches main or central source; many newly added chapter/fixture
paths are absent. This **does not prove every behavior is absent** or authorize
dropping/replacing the PRs. It establishes that shared-library progress is not
evidence these deltas have already been incorporated.

The early #693/#698/#707/#708/#709/#711/#712/#718/#725 slices also modify
modifier/tag/upgrade eligibility helpers. They are not documentation-only
changes. A grouped guide recovery must retain the associated source policy or
coordinate an explicitly equivalent replacement with those consumers.

## Shared APIs versus actual consumer migration

Five bounded shared guide services exist separately:

1. Restricted Document/style codec with actual native Text conversion.
2. Read-only native Font measurement, wrapping and logical style-hit facts.
3. Single post-native read-only CustomPayload observation.
4. Invocation-local current-book inspection and conditional native page turn.
5. Explicit caller-owned in-process request correlation.

Their ordinary-library plan/apply/type proofs and focused fixtures remain at
their own immutable cutoffs, recorded in the JSON. Counts overlap broader
suites and are not added together here. No direct Pluto service delegation was
found in the legacy TConstruct `*book*` modules at this source cutoff;
underlying resource helpers may separately use shared mechanics. Availability
of an API is not completion of its guide consumer migration.

TConstruct keeps chapters, item/book IDs, root/title/metadata schema, recipe and
modifier eligibility, source attribution, spread/layout, cover, navigation
history and action policy. Those must not move wholesale into PlutoLib.

## Navigation and LAN are separate unfinished boundaries

[#945](https://github.com/sofia-gros/horizon-mods/pull/945) removes the global
native page-turn hook and keeps guide navigation local to key/click/control
handlers. It preserves PageUp/PageDown and cover/spread/history/audio policy.
Its bounded source/helper checks and actual isolated plan plus11guards passed.
Native apply/typechecking, real input/audio/window and LAN remain unproved.
The full consumer plan's interrupted exit130 is inconclusive, not a pass.

The old materials-book-response HEAD is still enabled and conflicts with the
mandatory post-native payload observation contract. LAN refresh was not
disabled. A feature-preserving replacement still needs exact opening/attachment
ownership, retained overlay refresh, duplicate/stale handling, legacy envelope
and Text compatibility, and safe conditional update after read-only callbacks.
The valid262152-byte legacy envelope cannot silently be forced through the
ordinary262144-byte whole-payload observer limit.

[#944](https://github.com/sofia-gros/horizon-mods/pull/944) remains the v13 shared
snapshot, not automatically the later v14/v15 source or a full TConstruct proof.
Its guarded-source staging path is distinct from an ordinary launcher that does
not execute the mandatory Python guards.

## Excluded experiments and pending actual Game check

The [research checkpoint](https://github.com/sofia-gros/horizon-mods/commit/b434e9a4c11f355875ed0f3df38a2474d2f20478)
contains excluded correlation/lifetime models, an unapplied official11-file
storage proposal, and a separately sealed-v15 in-memory delta. The12model,
7generic-storage and21source-regression results do not compile the actual Game
field or prove native UI/packet behavior.

Actual Game lib/bin typing remains behind the metadata/capacity/serial gate.
The CI design owner currently prioritizes World's positive/negative borrow-only
probe; the guide storage patch is not in that job. Pristine and v15 dependency
graphs differ, so no fresh-runner or logical-path cache reuse is assumed.
No Client/renderer gate is bypassed for a positive result.

## Grouping and conditional retirement

A future broad category08 branch can group content, guide consumer adapters,
navigation and safe LAN recovery, with category01 shared APIs as dependencies.
Recipe-book discovery/index/placement/knowledge (#713/#943 and
#930/#935/#938/#941) belongs to the recipe/acquisition category and is only
cross-linked, not double-counted here.

A previous PR can be considered superseded only after an explicit grouped
replacement preserves its delta and dependencies, resolves old-hook conflicts,
passes the actual combined official gates and required gameplay/LAN cases, and
a separately authorized PR-state decision is made. **This inventory authorizes
no close, retarget, rebase, mark-ready, approval or merge action.**
