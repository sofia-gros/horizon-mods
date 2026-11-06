# Java and menu-word foundation composition

Status at source freeze: source composition checked; combined focused validation and native package verification are pending. The two earlier fixture proofs remain historical evidence and are not relabeled as a pass on this combined tree.

## Exact scope

Base: local `8715b1232c0fca133fe012f35830e2108e653d59`, remote `89744814b8d536a117e82f8ea3c8cdc9798faa95` (verified v15 foundation and its current documentation).

Selected groups only:
- Java runtime `18976165`, fixture correction `a92a41b5`, proof documentation `544419e7`; public checkpoint `b4be4b1347ad5168fc63a6227f9759ae55e30041`.
- Menu runtime `a709cb16`, proof documentation `74205318`; public checkpoint `d1c52d28a56696b5364246c03abaa658b0c2e454`.

Their union changes 27 paths before these integration notes. The only merge conflict was the inventory API module export list. Both exports and their original comments are retained. All other selected files equal their previously frozen group versions byte-for-byte. Existing central documentation is preserved; no older whole feature tree was copied.

The API remains one actual `pluto-inventory-api` package. No hook, state, registry, host package dependency, actor capability or automatic crafting connection is added. Java ID/sort/dedup/eligibility and menu routing/reset/commit/lifecycle policy stay in TConstruct.

## Workflow boundary

Only the already reviewed ordinary-test replacement from standalone `rustc` to the locked/offline real Cargo builder-wire fixture is included. The independent 17-path manual CPU/Game-borrow checkpoint `9385b77a37f19fab517b5a1ec09b4e1f8c824408` is not imported or modified. A later composition must preserve both changes in their respective jobs. No Actions run was dispatched or inspected here.

## Validation gates

1. Source identity, real-package paths and legacy oracles.
2. Java/menu Python boundary guards and deterministic native archive packaging, verifying both source modules appear exactly once and are byte-identical.
3. The two real Cargo fixtures plus the complete inventory API tests on this exact combined source, under the shared serial slot and continuous disk guard.
4. A complete official native apply/type gate is a separate coordinator-owned gate; do not infer it from pure-crate or packaging tests.

Keep the prior limitations: no new live Java reference run, no gameplay/network lifecycle or successful component-sensitive placement proof, no same-pair automatic crafting authority, and no consumer-wide migration claim.
