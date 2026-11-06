# Recipe, advancement and adjacent transport boundary inventory

A source-only review aid at central MOD `8715b1232c0fca133fe012f35830e2108e653d59`. This is not an audit of every original PR and not a completion percentage. Counts describe the 28 boundaries listed below, not PRs or finished gameplay features.

Classification: 17 shared-service boundaries, 8 retained consumer-policy boundaries, 3 unresolved native/lifecycle boundaries, **zero wrappers proved safe to remove unconditionally**.

The central v15 ordinary-library proof remains pinned to runtime `f5a83539`; the Java and menu codecs have separately published focused proofs and are not part of this audited central tree. Existing test counts overlap and must not be summed. No new Rust/API/native/CI/game/network run was performed for this inventory.

## Review buckets, not a final PR plan

- Advancement/acquisition: metadata startup, callbacks, explicit events, tag/index publication, tree projection and the corresponding TConstruct criteria.
- Recipes/crafting: preview, vetoes, index staging, ingredient parsing, saved knowledge, pair staging and source recipe ordering.
- Adjacent shared transport: existing menu codec plus its dependency-aware fixture/CI adoption. Do not create a separate PR for every helper.

The coordinator owns the repository-wide 5–10-category discussion. This document neither closes existing PRs nor decides their final grouping.

## Shared service

| ID | Boundary | Present scope and next review |
|---|---|---|
| S01 | [Inventory metadata and component event keys](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/advancements.rs) | Default-off generic metadata scope and component-aware inventory event adapter are in the central foundation. Next: Verify startup opt-in before first event in the actual consumer bundle, preserving native authority/progress. |
| S02 | [Item predicate callback dispatch](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/advancements/predicates.rs) | One registry, lossless component facts and explicit Unsupported/Handled/Conflict; no TConstruct predicate interpretation. Next: Port the feature-branch predicates and remove competing direct hooks in one coherent consumer review. |
| S03 | [Synchronous custom advancement events](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/advancements/events.rs) | Borrowed EventSink uses native current-actor/player-condition/progress path; Submitted is not an award receipt. Next: Validate both successful local and authoritative guest open callers; no fabricated open event. |
| S04 | [Resolved tags and per-world candidate index](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/advancements/tags.rs) | Owned-empty/unavailable/unmanaged states and separate per-world cache preserve native no-provider fallback. Next: Connect ready snapshots and clear/invalidation lifecycle without bypassing exclusive-target guards. |
| S05 | [Loaded advancement-tree projection](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/advancements/projection.rs) | Normalized full-definition comparison and atomic loaded-tree replacement; no progress/event hash reset. Next: Keep criteria/rewards/IDs local and validate actual producer lifecycle. |
| S06 | [Recipe UI invalidation](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/recipe-book-invalidation.md) | Explicit shared reset of native presentation caches, without changing knowledge or current selection policy. Next: Adopt at the existing successful definition-publication tail, not earlier. |
| S07 | [Current-value recipe-index compare-and-apply](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/recipe_book/index.rs) | Exact records, explicit removals and caller-selected Append/FirstGap; no persistent owner token. Next: Stage crafting/meta/index together before no-fail publication; retain each family allocation policy. |
| S08 | [Explicit component-rich ghost preview](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/recipe_book/preview.rs) | Current-entry/type/fit checks and read-only native ghost projection, without placement or knowledge authority. Next: Keep preview opt-in separate from authoritative ID-only placement denial. |
| S09 | [Automatic placement veto](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/recipe_book/placement.rs) | Deny-only callback boundary protects local/LAN/native entry routes with mandatory composition guards. Next: Wire source policy without converting Continue into a grant or weakening denial on preview failure. |
| S10 | [Crafting-match veto](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/crafting/matching.rs) | Cheap-interest filtering, semantic input facts, preserved cropped holes/order and denial before assembly. Next: Keep copied result facts read-only; do not add output/remainder authority to this API. |
| S11 | [Station item-ID ingredient parsing](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/crafting/ingredients.rs) | Generic registry queries and Unsupported versus owned-empty/invalid results; native fallback without providers. Next: Retain Forge/schema/depth policy and component-sensitive restrictions in consumers. |
| S12 | [Explicit output/remainder pair staging](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/crafting/products.rs) | One synchronous staged pair uses full native clones, recursive wire refusal and codec eligibility; no live-slot authority. Next: Keep frozen explicit API while automatic consuming boundary remains blocked. |
| S13 | [Prepared-world saved knowledge handoff](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/recipe_book/handoff.rs) | Explicit capture/ready/restore mechanics with consumer-selected IDs; no automatic restore hook. Next: Verify exact prepared-world and actor/reset boundaries in the consuming workflow. |
| S14 | [Lazy three-valued predicate folds](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/inventory-api/src/predicate_fold.rs) | Three real callers delegate ordered decisive/unknown folds; strict station semantics remain separate. Next: Review preserved visitation alongside full parser integration, not just fold results. |
| S15 | [Opt-in Java linked-bin ordering](https://github.com/sofia-gros/horizon-mods/blob/5254ee9e8fc4a9fc7aecdd2b78438175ecc48a5c/pluto-lib/mixins/pluto_native_creative/inventory-api/src/java_hashmap.rs) | Published focused-tested checkpoint, not yet part of the audited central8715 tree; no native ordering replacement. Next: Integrate runtime18976165 plus test correction/proof; keep source ID/sort/dedup/eligibility local. |
| S16 | [Signed menu-word UTF-8 list codec](https://github.com/sofia-gros/horizon-mods/blob/d1c52d28a56696b5364246c03abaa658b0c2e454/pluto-lib/mixins/pluto_native_creative/inventory-api/src/menu_words.rs) | Published focused-tested checkpoint; unchanged actor/reset/commit/receive callers. Next: Integrate codec, actual-Cargo CI step and proof with both existing workflow changes preserved. |
| S17 | [Raw resource layers and tag graph](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/api/src/resources.rs) | Generic traversal/read-error/layer and graph mechanics already exist; no recipe/material interpretation. Next: Prefer existing services for future producers rather than another registry/resolver. |

## Consumer policy

| ID | Boundary | Present scope and next review |
|---|---|---|
| P01 | Tool and tank predicate meaning | Feature branch owns component keys, modifier/material/tank rules and fail-closed missing-data decisions. Next: Keep these modules in TConstruct; do not infer central activation from their existence. |
| P02 | Acquisition tag selection and readiness | Managed-tag allowlist, source completeness and resolution remain consumer responsibilities. Next: Publish only a correctly scoped ready snapshot; absent state is not empty membership. |
| P03 | Container-open trigger/type mapping | Consumer maps successful smeltery/foundry opens to its own trigger and condition schema. Next: Do not move content IDs into Pluto or emit on failed/predicted opens. |
| P04 | [Discovery definitions and rewards](https://github.com/sofia-gros/horizon-mods/blob/8c56bb52b6b77fae6115e894af9e369a5c3f9e08/tconstruct/mixins/tconstruct/source-fake-material-discovery.rs) | Recipe-family IDs, criteria/rewards and grant/restore selection remain local. Next: Use generic projection/event mechanisms while keeping full eligibility decisions local. |
| P05 | [Recipe/schema/unknown and source order policy](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-conditions.rs) | Forge forms, empty-array handling, strict unknown semantics, ID composition and material eligibility are distinct local rules. Next: Do not unify strict station matching with decisive logical folds, or Java ordering with current native registration. |
| P06 | [Family-specific index and placement choice](https://github.com/sofia-gros/horizon-mods/blob/8c56bb52b6b77fae6115e894af9e369a5c3f9e08/tconstruct/mixins/tconstruct/source-fake-material-world.rs) | Fake-material append and preview-only behavior differ from Travelers gap reuse and ordinary placement. Next: Preserve both choices during staged index adoption; never enable ID-only component-sensitive placement. |
| P07 | [Overslime materials/output/remainders](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/source-overslime-crafting.rs) | Owned custom_data schema, refill arithmetic, material selection and output choice remain TConstruct logic. Next: Use explicit pair staging only; do not reconstruct full stacks from semantic component facts. |
| P08 | [Menu routing and snapshot actor/cadence policy](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/tconstruct/mixins/tconstruct/part-selection-send-data.rs) | Data-slot ranges, reset/commit, parsing, actor/generation/cache keys and refresh interval remain local. Next: Keep these unchanged until lifecycle-specific proof exists; the codec is not a receiver transaction. |

## Blocked / unproved

| ID | Boundary | Present scope and next review |
|---|---|---|
| B01 | [One authoritative automatic pair consumption](https://github.com/sofia-gros/horizon-mods/blob/20b926bc3fd521b56d53ec3d06a701ec74497df6/pluto-lib/docs/crafting-products-automatic-route-audit-20261106.md) | Three attempted receipt/interior transforms were rejected; different no-op HEAD probes resolve entry points only. No same-pair operation covering normal click/quick move/LAN/crafter is proved. Next: Retain unapplied minimum contract; do not enable speculative hooks or independent result/remainder recomputation. |
| B02 | [Persistent RecipeIndex ownership across reload/clone](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/recipe-book-preview.md) | Current-value comparison cannot distinguish identical reloads; supported lifecycle/state ownership remains unproved. Next: Use invocation-local CAS only; no prefixes, raw pointers or cached hashes as ownership proof. |
| B03 | [Dynamic recipe guest publication/lifecycle](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/recipe-index-patch.md) | Native source-file synchronization and distinct LAN display index do not prove publication of in-memory projected records. Next: Validate complete reload/listener/order/guest lifecycle separately from local index staging. |

## Removal is conditional, not an instruction

No whole wrapper is proved unused or safe to delete now. Thin codec adapters can remain useful stable consumer boundaries; `recipe-order.rs` retains real policy. Candidates for later retirement are:

- **Original direct ItemView/World predicate hooks:** Select and activate the stable callback consumer, preserve conflicts/default fallback, then remove competing target hooks in the same reviewed consumer bundle.
- **Old TConstruct native tag/index replacement bodies:** Publish the complete ready tag snapshot and prove reset/invalidation; retain the mandatory exclusive-target composition guard.
- **Old recipe preview/index/UI native boilerplate:** Port exact family selection and all-or-nothing publication first. Retain family policy and ID-only denial; do not simply delete the complete wrapper.

Do not remove deny-only guards because preview succeeds, replace native authority with value equality, or reactivate original hooks beside an exclusive shared replacement. Consumer migration/startup is still a separate step, not automatically covered by a library type-check.

## Deferred source candidates

- **Changed-or-expired byte snapshot refresh:** Exact duplicate exists in broad/Haste; defer new small API while coherent feature review is prioritized. No implementation started.
- **Indexed menu-word receive assembly:** Five repeated append/mismatch paths exist, but migration would touch actor/reset/commit lifecycle; no new owner/state is justified by duplication alone. No implementation started.

## Evidence and limits

The [machine-readable inventory](recipe-advancement-boundary-inventory-20261106.json) records exact MOD commit/path/SHA-256 evidence for every row, including separate acquisition feature commits and original fake-material/Travelers consumers. Those source branches are not silently included in the central foundation. The two acquisition prototype commits did not resolve remotely during this audit, so their records have no public source links. Published links use verified remote commits, which can differ from local proof commit IDs.

Source evidence was read only from horizon-mods and the already established official Horizon/Mixins evidence. No fork was accessed. No official code was changed or contacted. No PR was closed or merged. The next useful validation is a coherent combined consumer review and the outstanding native authority/lifecycle proof, not another small resend helper.
