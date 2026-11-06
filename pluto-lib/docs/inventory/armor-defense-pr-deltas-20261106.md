# Armor, defense and movement PR delta ledger

**Read-only inventory. No PR action, production activation or native verification.**

This is a work allocation of 59 fixed PRs, not a final category decision. Combat, tools, recipe/acquisition and shared-service changes retain explicit cross-links. A separate 23-row supplement fills the actual merge-base facts for the existing world-fluid ledger.

## Architecture and current validation boundary

The target layering is Horizon → horizon-mixins → the Mixins mod. A mod that chooses PlutoLib uses its shared APIs/adapters above Mixins; a mod that does not use PlutoLib may target Mixins directly. TConstruct is a PlutoLib consumer. PlutoLib is reusable infrastructure, not a mandatory loader or a dependency for every mod.
Actions-dependent native demonstrations are currently waiting because the user cannot operate Actions. This inventory does not infer that queued API/borrow/renderer experiments ran, and it provides no local-native fallback result. Generic terrain/provider/flow/tint/upward work is a separate group outside these fixed-head audit cutoffs.

## Immutable cutoffs

- MOD main: `08afa992d5091e03360f3473ea93fd17aa793987`.
- Shared foundation: local `8715b1232c0fca133fe012f35830e2108e653d59`, published equivalent `89744814b8d536a117e82f8ea3c8cdc9798faa95`.
- Official target: Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`, Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`.
- Fixed 265-PR metadata: the published `4bf5fe7d` reorganization snapshot; the companion JSON records its full SHA256. No PR metadata/state was rewritten.

## Findings

- All 59 exact heads are absent from fixed main and central ancestry. This does not exclude independently equivalent or squashed source.
- 47 rows inherit other fixed PR heads at or before their actual merge-base. None contains another fixed head inside its incremental range.
- The incremental ranges contain 62 unique commits. No whole-diff stable patch-id duplicates, shared exact incremental commits, or equivalent single-commit patches were found in this set.
- 70 paths are touched by more than one incremental delta. Path overlap is not duplicate implementation.
- Only #694 has a non-ancestor reported base: metadata `19378cf3d5d63bfac50fed3d68599fdf1c800dee` versus actual merge-base `5ac73ffeef51e25ea868801f9c211810249ff2b5`; the reported base has 33 commits not in the branch-side range. Never import a two-tree base-to-head reversal as its own feature patch.
- Earlier exact-head classifications remain 42 needs-fix / 17 unverified. They are not new test outcomes and are not promoted by shared-library evidence.
- 15 rows contain existing Pluto references in changed helper files, but only 0 rows add such reference lines in the actual delta. The detected namespaces concern creative/tank mechanics, not adoption of the new defense callbacks. The scoped defense facade search found no hits; aliasing is outside this lexical check.
- No changed native helper is byte-identical to the same path at central8715. That is not proof of semantic incompatibility; each consumer needs a reviewed port rather than automatic replacement.
- All 59 manifests still declare the older `0c7b5b61` target. Original-head compatibility remains separate from the v15 shared package.

## Delta and dependency semantics

For each PR, the JSON separates the GitHub-reported base SHA, actual `git merge-base --all(base, head)`, parent-open-PR metadata, branch-side incremental range, cumulative branch range relative to fixed main, and direct main-tree difference count. It records full raw status/mode/blob IDs with rename detection disabled, binary-patch SHA256, stable patch ID, exact commits and inclusion in main/central.
The cumulative raw-record checksum is SHA256 over sorted-key compact JSON; cumulative paths remain listed. Exact active helper entries and added/changed/removed manifest declarations are recorded with per-entry hashes. Historical blocker entry IDs are marked introduced/changed by this PR or inherited unchanged. Same-named active/disabled/absent central entries are evidence, not automatic compatibility. In total, 135 distinct changed helper paths are absent at the central cutoff; code retained only on feature branches must not be counted as migrated production.

## Contract mapping

### equipment-facts

- PRs: #697, #724, #742, #745, #755, #764, #788, #793, #812, #871, #873, #874, #890, #910, #917.
- Shared: Captured native equipment and shared semantic ItemFacts; optional read-only current actor/dimension fields.
- Keep in TConstruct: TConstruct trait selection, eligible equipment, attribute IDs, formulas and caps.
- Still unproved/unmigrated: No general attribute-write/refresh lifecycle, persistent actor token, current active-use field in the earlier protection snapshot, or equipment commit authority.
- Evidence: [equipment](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/damage-equipment-context.md), [protection](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/protection-provider-facade.md), [blocking](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/block-feedback.md).

### protection-contributions

- PRs: #897, #898, #899, #901, #902, #903, #905, #906, #907, #911, #912, #915.
- Shared: Single provider registry; source catalog facts and scoped lower/upper/additive protection contributions in guarded official hit paths.
- Keep in TConstruct: Damage type/tag selection, per-part and max-equipment calculations, signed altitude terms, attacker criteria and modifier budgets.
- Still unproved/unmigrated: Original StatusFlags extension/minimum/maximum hooks are not migrated by linking this library. TConstruct registration/formulas and each environmental/attacker path require separate source and runtime proof.
- Evidence: [protection](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/protection-provider-facade.md), [equipment](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/damage-equipment-context.md), [tag-graph](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/tag-graph.md).

### block-observation

- PRs: #788, #851, #865, #940, #942.
- Shared: Synchronous read-only final raised-item feedback and equipment view before native after_block.
- Keep in TConstruct: Shield eligibility, blockade/parrying/blocking values, source durability retention, feedback and ability rules.
- Still unproved/unmigrated: No replacement of shield wear, sound, disable or caller mutation; the old direct after_block injection cannot remain beside a new adapter without complete-plan review.
- Evidence: [blocking](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/block-feedback.md), [equipment](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/damage-equipment-context.md).

### armor-observation-and-wear

- PRs: #916, #917, #923, #925, #929, #931, #933, #934, #937, #939.
- Shared: Guarded native hurt_armor before/after observation, semantic equipment projection and reusable pure storage primitives.
- Keep in TConstruct: Tanned/Dense/Reinforced/Unbreakable/Solar/Overslime/retained-break formulas and ordered commit behavior.
- Still unproved/unmigrated: Observation is after native break/refresh, not final per-piece loss interception, mutation permission or rollback. Actual slot/actor authority and local/LAN retention remain unrecovered.
- Evidence: [armor-observation](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/armor-wear-observation.md), [wear-authority](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/equipment-authority-audit-20261105.md), [transactions](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/plain-stack-cells.md).

### motion-fluid-facts

- PRs: #694, #742, #745, #755, #764, #793, #812, #842, #871, #874, #890, #892, #896, #907, #910, #920.
- Shared: Built-in fluid/player/motion and climbability fact adapters; explicit custom-fluid immersion kernel/view.
- Keep in TConstruct: Gravity, jump counts, glide, movement/attack speeds, safe fall and source eligibility.
- Still unproved/unmigrated: No attribute application, same-frame motion mutation, jump transport/handshake, client prediction or native StatusFlags extension recovery.
- Evidence: [native-facts](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/native-fluid-facts.md), [curatives](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/immersion-curatives.md).

### effect-catalog-and-values

- PRs: #724, #749, #761, #788, #893, #894, #895, #904.
- Shared: Read-only native potion/effect descriptors and immutable caller-chosen per-instance curative policy values.
- Keep in TConstruct: Immunity and harmful-duration formulas, ignition/contact timing, milk/default curative choices and application policy.
- Still unproved/unmigrated: No native EffectInstance attachment, metadata copy/save propagation or same-call Game mutation authority. Unapplied research is excluded from production migration.
- Evidence: [potion-facts](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/potion-facts.md), [curatives](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/immersion-curatives.md).

### acquisition-and-products

- PRs: #865, #871, #873, #874, #875, #887, #888, #889, #891, #909, #913, #940, #942.
- Shared: Shared read-only item/component facts, crafting product/remainder proposals and borrowed plain-cell count operations.
- Keep in TConstruct: Recipes, modifier IDs, ranks, defense/ability slot budgets, extraction/salvage and menu presentation.
- Still unproved/unmigrated: No automatic station consumption/recipe activation, authoritative live equipment/menu writeback or permission from a proposal value alone.
- Evidence: [crafting-products](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/crafting-products.md), [transactions](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/plain-stack-cells.md), [migration](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-api-migration-status.md).

### resource-tags

- PRs: #897, #899, #901, #902, #903, #912, #915, #916.
- Shared: Generic raw resource layering/tag graph traversal and native source tag facts.
- Keep in TConstruct: TConstruct namespaces, damage/source taxonomy, material eligibility and branch-specific cause matching.
- Still unproved/unmigrated: Exact legacy collectors and consumers must explicitly adopt the service; a common file path or changed tag parser is not semantic equality.
- Evidence: [tag-graph](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/tag-graph.md), [protection](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/protection-provider-facade.md).

### combat-use-phase

- PRs: #908.
- Shared: Existing native read-only equipment/damage facts can supply explicit inputs only.
- Keep in TConstruct: Bonking draw/charge/release, main-hand attack strength, extra damage and knockback.
- Still unproved/unmigrated: No generic mutable attack transaction, use-session lifetime, multi-target reentrancy or same-turn commit authority has been proved for this PR.
- Evidence: [equipment](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/damage-equipment-context.md), [blocking](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/block-feedback.md), [wear-authority](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/equipment-authority-audit-20261105.md).

## Important introduced-versus-inherited cases

- #842 introduces three StatusFlags state fields and a jump/LAN pipeline, while inheriting the older Dreambound TAIL injection. #920 changes its jump levels/motion/wire consumers without introducing another native declaration. A generic motion-facts helper does not replace the pipeline.
- #851 introduces the direct after_block hook with the known destructured-parameter constraint. #865 and #940/#942 add acquisition/consumer rules; do not attribute the inherited hook defect to their own recipe changes or discard those rules with the hook.
- #897 introduces StatusFlags protection state; #898 replaces its fire-only entries with general protection entries. #899/#901 extend the formulas/tags without new manifest entries. #904 changes fire-duration state ownership to StatusFlags, rather than repairing native support merely because its title says supported constructor.
- #905 introduces a minimum_protection target and #911 introduces maximum_protection, absent from the selected official API. Later source-policy changes inherit those dependencies. Pluto lower/upper contributions are the migration candidate, not proof that those published heads use it.
- #923 changes native armor-wear interception and #925 adds pre-wear Tanned. #929/#931/#933/#934/#937/#939 extend ordered source wear/retention policy. Existing Pluto post-native observations do not supply the pre-wear per-piece commit contract.
- #908 is Bonking combat/use-session work despite this audit assignment. Preserve its combat/tools cross-link; no generic mutable attack capability follows from equipment facts.

## Per-PR index

The full rows, exact SHAs, blobs, helper references and edges are in the adjacent JSON. `parent` means the fixed open-PR base-ref match, not a complete semantic dependency.

- [#694](https://github.com/sofia-gros/horizon-mods/pull/694) feat(tconstruct): connect Unburdened use-item movement speed
  - head `037c0b48f90f992051e56f038c3b642da60e97e5`; merge-base `5ac73ffeef51e25ea868801f9c211810249ff2b5`; parent none; 8 incremental / 8 cumulative paths; 1 commit(s).
  - Boundaries: motion-fluid-facts. Historical classification: needs_fix.
- [#697](https://github.com/sofia-gros/horizon-mods/pull/697) feat(tconstruct): apply worn Revitalizing health trait
  - head `787911c89a828e17a1726e77c9144248ea5bf6a3`; merge-base `19378cf3d5d63bfac50fed3d68599fdf1c800dee`; parent none; 6 incremental / 6 cumulative paths; 2 commit(s).
  - Boundaries: equipment-facts. Historical classification: unverified.
- [#724](https://github.com/sofia-gros/horizon-mods/pull/724) Connect bounded worn Crystalstrike effects
  - head `3acf5f05df2f660cd2092ed123f303b5ea0482d6`; merge-base `6d980fa32cf9295c54845946d56ba2340c10a327`; parent none; 10 incremental / 10 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, effect-catalog-and-values. Historical classification: needs_fix.
- [#742](https://github.com/sofia-gros/horizon-mods/pull/742) Connect worn Cobalamin attack and mining attributes
  - head `a0a53cee91965adf5dfce2f84c7d002553c3b5b5`; merge-base `6d980fa32cf9295c54845946d56ba2340c10a327`; parent none; 10 incremental / 10 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, motion-fluid-facts. Historical classification: needs_fix.
- [#745](https://github.com/sofia-gros/horizon-mods/pull/745) Connect worn Godspeed attack movement and mining attributes
  - head `70a4a279e0e808cc133939505976210c58cad800`; merge-base `a0a53cee91965adf5dfce2f84c7d002553c3b5b5`; parent #742; 5 incremental / 12 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, motion-fluid-facts. Historical classification: needs_fix.
- [#749](https://github.com/sofia-gros/horizon-mods/pull/749) Connect worn Shell Gut poison hunger and nausea immunity
  - head `81b1c25e71774716eb5530c6f178040e7904c88e`; merge-base `6d980fa32cf9295c54845946d56ba2340c10a327`; parent none; 7 incremental / 7 cumulative paths; 1 commit(s).
  - Boundaries: effect-catalog-and-values. Historical classification: unverified.
- [#755](https://github.com/sofia-gros/horizon-mods/pull/755) Connect worn Dragonfall critical hits and safe fall bonus
  - head `42d95e5c26e715e16b5f7953aafebbdf4dd6e4e0`; merge-base `6d980fa32cf9295c54845946d56ba2340c10a327`; parent none; 8 incremental / 8 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, motion-fluid-facts. Historical classification: needs_fix.
- [#761](https://github.com/sofia-gros/horizon-mods/pull/761) Connect worn Balm harmful-effect duration reductions
  - head `06ed3264b37feecf647fb593b418da445e4fe0d1`; merge-base `3acf5f05df2f660cd2092ed123f303b5ea0482d6`; parent #724; 7 incremental / 13 cumulative paths; 1 commit(s).
  - Boundaries: effect-catalog-and-values. Historical classification: needs_fix.
- [#764](https://github.com/sofia-gros/horizon-mods/pull/764) Connect exact-level worn Dreambound gravity
  - head `6553230778d7e6f9aba497bcdecc78e5a66bc55f`; merge-base `6d980fa32cf9295c54845946d56ba2340c10a327`; parent none; 7 incremental / 7 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, motion-fluid-facts. Historical classification: needs_fix.
- [#788](https://github.com/sofia-gros/horizon-mods/pull/788) Connect Crystalstrike shield effects in both hands
  - head `a6d8eb53f9a76421e2ce01aa93ef095d1e48a964`; merge-base `06ed3264b37feecf647fb593b418da445e4fe0d1`; parent #761; 6 incremental / 15 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, block-observation, effect-catalog-and-values. Historical classification: needs_fix.
- [#793](https://github.com/sofia-gros/horizon-mods/pull/793) Connect worn and held Ricochet knockback strength
  - head `672bc05cfc9463e19bd99ef99ececdb042898f05`; merge-base `6d980fa32cf9295c54845946d56ba2340c10a327`; parent none; 6 incremental / 6 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, motion-fluid-facts. Historical classification: unverified.
- [#812](https://github.com/sofia-gros/horizon-mods/pull/812) Connect worn Skyfall gravity and shared safe fall distance
  - head `24f6b3beb44985456417ee081c788c0d1f6e4ff8`; merge-base `42d95e5c26e715e16b5f7953aafebbdf4dd6e4e0`; parent #755; 8 incremental / 12 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, motion-fluid-facts. Historical classification: needs_fix.
- [#842](https://github.com/sofia-gros/horizon-mods/pull/842) Connect bounded worn Dreambound extra jumps
  - head `652f02fdc32b445eae02b35a7fd2fc1d49e479fb`; merge-base `6553230778d7e6f9aba497bcdecc78e5a66bc55f`; parent #764; 16 incremental / 20 cumulative paths; 3 commit(s).
  - Boundaries: motion-fluid-facts. Historical classification: needs_fix.
- [#851](https://github.com/sofia-gros/horizon-mods/pull/851) feat: implement source stock shield blocking (#379)
  - head `dc3f7e5ae8a2ab6916d1cdcb2295bb5757232460`; merge-base `6d980fa32cf9295c54845946d56ba2340c10a327`; parent none; 9 incremental / 9 cumulative paths; 1 commit(s).
  - Boundaries: block-observation. Historical classification: needs_fix.
- [#865](https://github.com/sofia-gros/horizon-mods/pull/865) feat(tconstruct): enable Blockade acquisition and fractional shield strength
  - head `b5302420a922313b1a70a811c585c855e80a2006`; merge-base `dc3f7e5ae8a2ab6916d1cdcb2295bb5757232460`; parent #851; 8 incremental / 14 cumulative paths; 1 commit(s).
  - Boundaries: block-observation, acquisition-and-products. Historical classification: needs_fix.
- [#871](https://github.com/sofia-gros/horizon-mods/pull/871) feat(tconstruct): add Respiration acquisition and breathing
  - head `07d3e66e4d9ea837f5ebf58384521732b103b6e8`; merge-base `6d980fa32cf9295c54845946d56ba2340c10a327`; parent none; 10 incremental / 10 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, motion-fluid-facts, acquisition-and-products. Historical classification: unverified.
- [#873](https://github.com/sofia-gros/horizon-mods/pull/873) feat(tconstruct): add bounded Knockback Resistance defense upgrade acquisition
  - head `4539e503683e0d011e6b58d23aaa9c449043918d`; merge-base `6d980fa32cf9295c54845946d56ba2340c10a327`; parent none; 7 incremental / 7 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, acquisition-and-products. Historical classification: unverified.
- [#874](https://github.com/sofia-gros/horizon-mods/pull/874) feat(tconstruct): add bounded Depth Strider boot upgrades and water movement
  - head `338124697ccbbccb1d8b709faed28ed03cd0e4e4`; merge-base `6d980fa32cf9295c54845946d56ba2340c10a327`; parent none; 10 incremental / 10 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, motion-fluid-facts, acquisition-and-products. Historical classification: unverified.
- [#875](https://github.com/sofia-gros/horizon-mods/pull/875) feat(tconstruct): apply Fortified material defense slots to station capacity
  - head `14244b52399bdc5dd26948544a64893449c0a1a8`; merge-base `4539e503683e0d011e6b58d23aaa9c449043918d`; parent #873; 9 incremental / 11 cumulative paths; 1 commit(s).
  - Boundaries: acquisition-and-products. Historical classification: unverified.
- [#887](https://github.com/sofia-gros/horizon-mods/pull/887) feat(tconstruct): add bounded Double Jump boot ability acquisition and extraction
  - head `5ebb007566a0297a0b387015b8c1145895821717`; merge-base `338124697ccbbccb1d8b709faed28ed03cd0e4e4`; parent #874; 7 incremental / 12 cumulative paths; 1 commit(s).
  - Boundaries: acquisition-and-products. Historical classification: unverified.
- [#888](https://github.com/sofia-gros/horizon-mods/pull/888) feat(tconstruct): add Wings ability acquisition with two-slot accounting
  - head `c87c90b79c3435fe04d1befc7ebc3ef6d7d790df`; merge-base `5ebb007566a0297a0b387015b8c1145895821717`; parent #887; 7 incremental / 14 cumulative paths; 1 commit(s).
  - Boundaries: acquisition-and-products. Historical classification: unverified.
- [#889](https://github.com/sofia-gros/horizon-mods/pull/889) feat(tconstruct): add remaining Protection acquisitions with shared defense budgeting
  - head `4849df7de5f73fa75df813649ef35da127a71ed8`; merge-base `cf8a2249d4ff17bb6c8c4e6cc42b317545ddb9a6`; parent #891; 8 incremental / 14 cumulative paths; 1 commit(s).
  - Boundaries: acquisition-and-products. Historical classification: unverified.
- [#890](https://github.com/sofia-gros/horizon-mods/pull/890) feat(tconstruct): apply Projectile Protection maximum-equipped knockback resistance
  - head `bcc431506fd3e7b5d503876074f35d8a24cd92ff`; merge-base `4849df7de5f73fa75df813649ef35da127a71ed8`; parent #889; 8 incremental / 18 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, motion-fluid-facts. Historical classification: unverified.
- [#891](https://github.com/sofia-gros/horizon-mods/pull/891) feat(tconstruct): add Fire Protection increments and shared defense slot accounting
  - head `cf8a2249d4ff17bb6c8c4e6cc42b317545ddb9a6`; merge-base `14244b52399bdc5dd26948544a64893449c0a1a8`; parent #875; 7 incremental / 13 cumulative paths; 1 commit(s).
  - Boundaries: acquisition-and-products. Historical classification: unverified.
- [#892](https://github.com/sofia-gros/horizon-mods/pull/892) feat(tconstruct): connect acquired Melee Protection progress to use-item movement speed
  - head `469014ae90ef46e33fe7fa8ae9fa3c8a6934ed76`; merge-base `037c0b48f90f992051e56f038c3b642da60e97e5`; parent #694; 4 incremental / 8 cumulative paths; 1 commit(s).
  - Boundaries: motion-fluid-facts. Historical classification: unverified.
- [#893](https://github.com/sofia-gros/horizon-mods/pull/893) feat: apply Fire Protection to explicit player ignition
  - head `89da5b453c2b966096167ec20948ae844171dc50`; merge-base `bcc431506fd3e7b5d503876074f35d8a24cd92ff`; parent #890; 9 incremental / 24 cumulative paths; 1 commit(s).
  - Boundaries: effect-catalog-and-values. Historical classification: unverified.
- [#894](https://github.com/sofia-gros/horizon-mods/pull/894) feat: apply Magic Protection harmful-effect duration
  - head `b267dcd101034da0174c24fc549e4189c7b076ac`; merge-base `a6d8eb53f9a76421e2ce01aa93ef095d1e48a964`; parent #788; 8 incremental / 18 cumulative paths; 1 commit(s).
  - Boundaries: effect-catalog-and-values. Historical classification: needs_fix.
- [#895](https://github.com/sofia-gros/horizon-mods/pull/895) feat: apply Fire Protection contact duration and repair ignition hook
  - head `64e5c4c6fe1e100e57b47cdff9df8572cd4b2634`; merge-base `89da5b453c2b966096167ec20948ae844171dc50`; parent #893; 8 incremental / 26 cumulative paths; 1 commit(s).
  - Boundaries: effect-catalog-and-values. Historical classification: unverified.
- [#896](https://github.com/sofia-gros/horizon-mods/pull/896) feat: apply source Blast Protection explosion knockback
  - head `068efde25c4f214f69ac5e8a027348bc19a89e58`; merge-base `64e5c4c6fe1e100e57b47cdff9df8572cd4b2634`; parent #895; 7 incremental / 30 cumulative paths; 1 commit(s).
  - Boundaries: motion-fluid-facts. Historical classification: unverified.
- [#897](https://github.com/sofia-gros/horizon-mods/pull/897) feat: connect Fire Protection post-armor player damage
  - head `400d647dd7e7a0fdeb146aa9c2c484cfa839bc8f`; merge-base `068efde25c4f214f69ac5e8a027348bc19a89e58`; parent #896; 11 incremental / 37 cumulative paths; 1 commit(s).
  - Boundaries: protection-contributions, resource-tags. Historical classification: needs_fix.
- [#898](https://github.com/sofia-gros/horizon-mods/pull/898) feat: connect Blast Protection to shared post-armor damage
  - head `b3fb9279a6d7d5fc67c1921dcc2c7decf1f56b03`; merge-base `400d647dd7e7a0fdeb146aa9c2c484cfa839bc8f`; parent #897; 15 incremental / 38 cumulative paths; 1 commit(s).
  - Boundaries: protection-contributions. Historical classification: needs_fix.
- [#899](https://github.com/sofia-gros/horizon-mods/pull/899) feat: connect Projectile Protection post-armor damage
  - head `1da00a1768905a401388cd8f216f8a28862632a3`; merge-base `385dff4bc167da777d7dda01f805d0d802fe5bfe`; parent #901; 10 incremental / 40 cumulative paths; 1 commit(s).
  - Boundaries: protection-contributions, resource-tags. Historical classification: needs_fix.
- [#901](https://github.com/sofia-gros/horizon-mods/pull/901) feat: add Magic Protection to post-armor player damage
  - head `385dff4bc167da777d7dda01f805d0d802fe5bfe`; merge-base `b3fb9279a6d7d5fc67c1921dcc2c7decf1f56b03`; parent #898; 8 incremental / 39 cumulative paths; 1 commit(s).
  - Boundaries: protection-contributions, resource-tags. Historical classification: needs_fix.
- [#902](https://github.com/sofia-gros/horizon-mods/pull/902) feat: apply Melee Protection to verified native direct hits
  - head `be5200a4b0a179c55f983cb73f980d39f645d2b2`; merge-base `1da00a1768905a401388cd8f216f8a28862632a3`; parent #899; 10 incremental / 42 cumulative paths; 1 commit(s).
  - Boundaries: protection-contributions, resource-tags. Historical classification: needs_fix.
- [#903](https://github.com/sofia-gros/horizon-mods/pull/903) feat: block Rugged terrain and level-two attack damage
  - head `efa31e6f9b04a6c8caecd7c09373f01c33e2b1a9`; merge-base `4587e061b2fba57388aa582a7a2a529ea27f8ed0`; parent #904; 7 incremental / 48 cumulative paths; 1 commit(s).
  - Boundaries: protection-contributions, resource-tags. Historical classification: needs_fix.
- [#904](https://github.com/sofia-gros/horizon-mods/pull/904) fix: attach fire duration context to supported status constructor
  - head `4587e061b2fba57388aa582a7a2a529ea27f8ed0`; merge-base `be5200a4b0a179c55f983cb73f980d39f645d2b2`; parent #902; 5 incremental / 43 cumulative paths; 1 commit(s).
  - Boundaries: effect-catalog-and-values. Historical classification: needs_fix.
- [#905](https://github.com/sofia-gros/horizon-mods/pull/905) feat: apply Depth Protection bonuses and altitude penalties
  - head `52f21dd80b2fe485d396b6cedaa15cd52db167e5`; merge-base `efa31e6f9b04a6c8caecd7c09373f01c33e2b1a9`; parent #903; 9 incremental / 54 cumulative paths; 1 commit(s).
  - Boundaries: protection-contributions. Historical classification: needs_fix.
- [#906](https://github.com/sofia-gros/horizon-mods/pull/906) feat: connect Scorch Protection virtual fire enchantment
  - head `0e5018655d174c1cdb6d85063d6b8b5615c44a1d`; merge-base `52f21dd80b2fe485d396b6cedaa15cd52db167e5`; parent #905; 7 incremental / 57 cumulative paths; 1 commit(s).
  - Boundaries: protection-contributions. Historical classification: needs_fix.
- [#907](https://github.com/sofia-gros/horizon-mods/pull/907) feat: connect Turtle Shell per-part underwater protection
  - head `5592f15f69ea9840073d79952a91531242f163fd`; merge-base `0e5018655d174c1cdb6d85063d6b8b5615c44a1d`; parent #906; 8 incremental / 62 cumulative paths; 1 commit(s).
  - Boundaries: protection-contributions, motion-fluid-facts. Historical classification: needs_fix.
- [#908](https://github.com/sofia-gros/horizon-mods/pull/908) feat(tconstruct): implement main-hand Battlesign Bonking against mobs
  - head `b4e288a79777cc4bf999584c464490c5d6b889af`; merge-base `b5302420a922313b1a70a811c585c855e80a2006`; parent #865; 15 incremental / 28 cumulative paths; 1 commit(s).
  - Boundaries: combat-use-phase. Historical classification: needs_fix.
- [#909](https://github.com/sofia-gros/horizon-mods/pull/909) feat: enable Turtle Shell Station defense upgrade
  - head `12a53a00c2284d18fae9a32d27840fe4bfb7533a`; merge-base `5592f15f69ea9840073d79952a91531242f163fd`; parent #907; 8 incremental / 66 cumulative paths; 1 commit(s).
  - Boundaries: acquisition-and-products. Historical classification: needs_fix.
- [#910](https://github.com/sofia-gros/horizon-mods/pull/910) feat(tconstruct): project Turtle's Grace respiration
  - head `f031f6e38d6610e2cbbb1c787994030cb9367285`; merge-base `07d3e66e4d9ea837f5ebf58384521732b103b6e8`; parent #871; 6 incremental / 13 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, motion-fluid-facts. Historical classification: unverified.
- [#911](https://github.com/sofia-gros/horizon-mods/pull/911) feat(tconstruct): connect Boundless equipment protection cap
  - head `f2382cef6fde31d7b8c3523b0f1e714034c5e62d`; merge-base `12a53a00c2284d18fae9a32d27840fe4bfb7533a`; parent #909; 7 incremental / 71 cumulative paths; 1 commit(s).
  - Boundaries: protection-contributions. Historical classification: needs_fix.
- [#912](https://github.com/sofia-gros/horizon-mods/pull/912) feat(tconstruct): connect Consecrated armor protection
  - head `041a1145372640e73d6588af4383c65609308937`; merge-base `942c9acdeaa26d5104fbd167c7e548aa387d9c36`; parent #915; 8 incremental / 78 cumulative paths; 1 commit(s).
  - Boundaries: protection-contributions, resource-tags. Historical classification: needs_fix.
- [#913](https://github.com/sofia-gros/horizon-mods/pull/913) feat(tconstruct): add counted Boundless acquisition and salvage
  - head `afc0f85bb3a815cdb876d24d2e6e0751d466baf0`; merge-base `f2382cef6fde31d7b8c3523b0f1e714034c5e62d`; parent #911; 7 incremental / 72 cumulative paths; 1 commit(s).
  - Boundaries: acquisition-and-products. Historical classification: needs_fix.
- [#915](https://github.com/sofia-gros/horizon-mods/pull/915) feat(tconstruct): apply Scorch Protection to identified attackers
  - head `942c9acdeaa26d5104fbd167c7e548aa387d9c36`; merge-base `afc0f85bb3a815cdb876d24d2e6e0751d466baf0`; parent #913; 10 incremental / 75 cumulative paths; 1 commit(s).
  - Boundaries: protection-contributions, resource-tags. Historical classification: needs_fix.
- [#916](https://github.com/sofia-gros/horizon-mods/pull/916) feat(tconstruct): resolve Tanned live wear cause tags
  - head `4cb3d4e5ef2ad16b3890394143006bc1358d46db`; merge-base `041a1145372640e73d6588af4383c65609308937`; parent #912; 6 incremental / 83 cumulative paths; 1 commit(s).
  - Boundaries: armor-observation-and-wear, resource-tags. Historical classification: needs_fix.
- [#917](https://github.com/sofia-gros/horizon-mods/pull/917) feat(tconstruct): refresh Maintained armor toughness from wear
  - head `a14d1289a8877709bf4b8d30087ecabbaa63afcb`; merge-base `4cb3d4e5ef2ad16b3890394143006bc1358d46db`; parent #916; 10 incremental / 90 cumulative paths; 1 commit(s).
  - Boundaries: equipment-facts, armor-observation-and-wear. Historical classification: needs_fix.
- [#920](https://github.com/sofia-gros/horizon-mods/pull/920) feat(tconstruct): connect Double Jump to the shared airborne budget
  - head `5bba29b55b579f45252525eb0fd4e79c45d20dfc`; merge-base `652f02fdc32b445eae02b35a7fd2fc1d49e479fb`; parent #842; 6 incremental / 21 cumulative paths; 1 commit(s).
  - Boundaries: motion-fluid-facts. Historical classification: needs_fix.
- [#923](https://github.com/sofia-gros/horizon-mods/pull/923) fix(tconstruct): restore bounded retained armor wear writeback
  - head `1087425047f60b7e39394d3b0f7306cc41260e6b`; merge-base `a14d1289a8877709bf4b8d30087ecabbaa63afcb`; parent #917; 7 incremental / 96 cumulative paths; 1 commit(s).
  - Boundaries: armor-observation-and-wear. Historical classification: needs_fix.
- [#925](https://github.com/sofia-gros/horizon-mods/pull/925) feat(tconstruct): apply Tanned before ordinary armor wear
  - head `fde0e587d1064516ca1b563284b30d1c38c69337`; merge-base `1087425047f60b7e39394d3b0f7306cc41260e6b`; parent #923; 5 incremental / 98 cumulative paths; 1 commit(s).
  - Boundaries: armor-observation-and-wear. Historical classification: needs_fix.
- [#929](https://github.com/sofia-gros/horizon-mods/pull/929) feat(tconstruct): apply Dense to ordinary armor wear
  - head `1a6ca384dda7f65405aad12ae2267a38f74e7694`; merge-base `fde0e587d1064516ca1b563284b30d1c38c69337`; parent #925; 5 incremental / 100 cumulative paths; 1 commit(s).
  - Boundaries: armor-observation-and-wear. Historical classification: needs_fix.
- [#931](https://github.com/sofia-gros/horizon-mods/pull/931) feat(tconstruct): restore ordered Reinforced and Unbreakable armor wear
  - head `21b945dfbe8848caf10a7fefc25859d55daa305a`; merge-base `1a6ca384dda7f65405aad12ae2267a38f74e7694`; parent #929; 7 incremental / 102 cumulative paths; 1 commit(s).
  - Boundaries: armor-observation-and-wear. Historical classification: needs_fix.
- [#933](https://github.com/sofia-gros/horizon-mods/pull/933) feat(tconstruct): apply Solar Powered to ordinary armor wear
  - head `e188a6058f5e50c5d246a10a9ecc4be2f8680aae`; merge-base `21b945dfbe8848caf10a7fefc25859d55daa305a`; parent #931; 5 incremental / 105 cumulative paths; 1 commit(s).
  - Boundaries: armor-observation-and-wear. Historical classification: needs_fix.
- [#934](https://github.com/sofia-gros/horizon-mods/pull/934) feat(tconstruct): commit ordinary armor Overslime absorption
  - head `3a32d936eff227c881d1ef327bbc5ba7c8468478`; merge-base `e188a6058f5e50c5d246a10a9ecc4be2f8680aae`; parent #933; 7 incremental / 107 cumulative paths; 1 commit(s).
  - Boundaries: armor-observation-and-wear. Historical classification: needs_fix.
- [#937](https://github.com/sofia-gros/horizon-mods/pull/937) feat(tconstruct): allow Overslime armor friend combinations
  - head `1c8f382783e9f3624246247748b36ff39b367855`; merge-base `3a32d936eff227c881d1ef327bbc5ba7c8468478`; parent #934; 4 incremental / 108 cumulative paths; 1 commit(s).
  - Boundaries: armor-observation-and-wear. Historical classification: needs_fix.
- [#939](https://github.com/sofia-gros/horizon-mods/pull/939) feat(tconstruct): show retained armor break feedback
  - head `95de8dbf79b6ee986d3379f7a8bd51f48a13e2c6`; merge-base `1c8f382783e9f3624246247748b36ff39b367855`; parent #937; 6 incremental / 110 cumulative paths; 1 commit(s).
  - Boundaries: armor-observation-and-wear. Historical classification: needs_fix.
- [#940](https://github.com/sofia-gros/horizon-mods/pull/940) feat(tconstruct): add Parrying acquisition and safe extraction
  - head `9c412ab7801f3bcab83c86a3c6231a10b28a5f1a`; merge-base `b5302420a922313b1a70a811c585c855e80a2006`; parent #865; 5 incremental / 15 cumulative paths; 1 commit(s).
  - Boundaries: block-observation, acquisition-and-products. Historical classification: needs_fix.
- [#942](https://github.com/sofia-gros/horizon-mods/pull/942) feat(tconstruct): add ordinary-tool Blocking acquisition
  - head `3a7988373b092e48a738125c51308c3ae3f61b70`; merge-base `9c412ab7801f3bcab83c86a3c6231a10b28a5f1a`; parent #940; 5 incremental / 16 cumulative paths; 1 commit(s).
  - Boundaries: block-observation, acquisition-and-products. Historical classification: needs_fix.

## World-fluid supplement

The separate `world_merge_base_supplement` array covers the original 23 fixed world PRs, without replacing [their content/contracts inventory](https://github.com/sofia-gros/horizon-mods/blob/5f6c20539d7d7740bf559d06de60b52f38f62053/pluto-lib/docs/inventory/world-fluid-reorganization-20261106.json). It records reported/actual base, incremental path count, full binary-diff SHA256, stable patch ID and main/central ancestry. No old fluid head is promoted to official-playable status.

## Conditional retirement / next owner decision

Do not close or rebase a PR on this report alone. First select a coherent consumer group, preserve its own data/formula/acquisition delta, replace conflicting old hooks with the exact shared contract, and run the complete consuming plan/type and required local/LAN/save/physics paths. Only after equivalent coverage is demonstrated may obsolete transport/state or direct host-hook portions be retired. Original content changes and shared mechanics may need different destination branches.

This audit used only MOD Git reads and existing pinned MOD evidence. It did not inspect the prohibited Horizon fork, apply an engine patch, run native code, inspect CI, or change PR state.
