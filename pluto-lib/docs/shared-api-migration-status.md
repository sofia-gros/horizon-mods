# Shared API migration ledger

This is Step 2, the shared-service migration, on official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. The
[complete-library report](shared-foundation-validation-20261105.md) identifies
the latest immutable package cutoff and its actual results. It does not certify
all original TConstruct PRs or complete Step 3 consumer activation and later
game/save/LAN validation. Supporting fixtures retain their own exact source
cutoffs. Their counts overlap and must not be summed as distinct new tests.

## Coherent feature groups

| Group | Shared implementation | Consumer and remaining boundary |
|---|---|---|
| Inventory/storage | Lossless native codec and canonical decode, observable snapshots, recursive opaque-wire checks, compound patches, partitioned proposals, indexed read projections and explicit borrowed plain-cell count mechanics. A separate borrowed native Wire reader/comparator includes retained decoded baselines. | Main machine callers mechanically adopt established services; three plain-cell wrappers delegate with separately documented malformed-zero rejection. The baseline comparator is additive and has no consumer activation. Ownership keys, aliases, raw-slot forms, permissions and authoritative commit remain local. Neither comparator grants live-write authority. |
| Defense/equipment | One host-independent provider registry, source/equipment facts, signed/additive/capped protection, exact guarded ordinary/explosion callers and read-only block/armor receipts | TConstruct formulas, modifier selection and wear policy remain outside Pluto. Live callback delivery, actor lifecycle and consumer wear need separate proof. Read-only facts do not grant mutation. |
| Advancement | Metadata scope, stable component predicates, explicit events, per-world tag/index cache and atomic loaded-tree projection | Feature-branch activation and old direct-hook removal are pending. No automatic rewards/progress edits or invented inventory event. |
| Resource/tag | Ordered raw layers, explicit read-error policy and generic tag graph traversal | Existing collectors delegate mechanics. Namespace rules, removals/redirects, recipes, material parsing and eligibility remain TConstruct-owned. |
| Fluid/render/catalog | Fluid registry/transactions/flow kernel, immutable native contact views, built-in fluid/tank/motion facts, exact crop/tile/cuboid primitives, potion/effect facts and explicit cloud construction/insertion capability, immutable resource-owned terrain binding catalogs fixed-size cell geometry, explicit immersion-height calculations and immutable curative policy values | Mechanical callers use facts/rendering. Real minimal-loader contact testing is separate from production assets. Full custom-fluid world meshing, effects and saves are not recovered. Cloud insertion is type-checked against actual native Ready/level/actor lookups but unactivated and not live-tested. Geometry is not connected to meshing. Curative values are not attached to native instances; same-pass contact timing remains unresolved. |
| Recipe book | Presentation invalidation, checked ghost preview/cell projection, saved knowledge capture/restore, current-value index compare-and-apply, deny-only placement and crafting-match gates, and bounded station ingredient parsing | Persistent ownership and reload identity remain unresolved. Index callers must stage crafting/meta/index together before publication. Preview never grants placement; veto policies are unactivated. Saved restore is opt-in, readiness-bound and not live-tested. |
| Guide | Restricted document codec, native Text conversion, bounded post-native read-only payload observation, conditional current-book inspection/page turning and explicit caller-owned request correlation | No persistent screen/session carrier, native stale-response authority or outbound transport. Request correlation is in-process only and requires caller invalidation; it grants no native ownership. Current-book comparison is invocation-local and does not attest admitted input. Native book restoration, identical reopening and same-world player reset require explicit lifecycle handling. |
| Common integration | Official pin pair, recursive source packaging, one registry owner per service, mandatory source/composition guards and pinned test dependencies | Only guarded source-tree staging is supported. The ordinary official launcher does not execute these guards. Library type-checks are not full consuming-mod proof. |

## Critical dependency order

1. Keep APIs generic and no-consumer behavior explicit. Native snapshots and
   callbacks must not silently become authority or write tokens.
2. Prove remaining lifecycle and host seams against actual official sources.
   Current work includes guide session/reset ownership, custom-fluid mesh hookup,
   contact effects/curative metadata, recipe output/remainder authority and equipment
   commit phases. The isolated guide experiment passed type proof, but these are not completed features
   merely because an interface or small plan exists.
3. Port each coherent TConstruct feature group to the shared contract. Remove
   competing old hooks and wire opt-in startup registration. Preserve content
   IDs, formulas, layouts, metadata schemas and source policy. Any fallible
   replacement requires explicit all-or-nothing caller publication.
4. Validate the actual combined consuming bundle, then execute affected player,
   save/reload and multiplayer paths. Library-only and modeled fixture results
   cannot be relabelled as these outcomes.

This ledger is categorical rather than a percentage. Code volume and fixture
counts do not estimate remaining lifecycle work. The original PR classification
is a separate immutable audit; a shared fix does not retroactively validate an
original head.

## Known contract limits

- `same_item_snapshot` intentionally covers observable native state. The separate
  retained-baseline comparator additionally compares the private stored values;
  neither is a transaction, ownership, transport or general write capability.
  Saved encoding continues to reject all retained wire.
- Host-independent interfaces and host-qualified adapters are distinct. The two
  existing native API crates follow official Mixins dependency restrictions;
  no universal cross-host ABI or reverse game dependency is claimed.
- Every official upgrade must recheck immutable fingerprints and affected
  contracts. Pluto reduces consumer churn, not the need for version validation.
