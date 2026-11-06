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


## Combined-source API unit gate, 2026-11-06

The exact combined source `69c6b72b0b79b64317483bede6a2b2a689d5b83a` (remote exact-tree checkpoint `7aaf04c6091b74a6d343137a26aa95069c9b7e7a`) passed **125 API unit tests, 0 failed, 0 ignored**, using a complete unchanged copy of the real 30-file API package and committed lock seed. All 193 materialized source hashes, all API hashes and the seed lock were checked before and after. This is the actual combined API, not the older separate group results.

The single allowed test-harness compilation was observed through a fail-closed Rust compiler wrapper. It permitted exactly one `pluto_inventory_api --test` invocation at the copied real source path and correct output directory. Registry crates, another API compilation and non-test library compilation would have been rejected before compiler execution. The remaining calls were two Cargo version queries and one print-capability query. These incidental Cargo queries were not a manual version-check task.

The locked/offline, jobs=1, debug=0, incremental=0 run finished in 3.40 seconds. The continuous floor was 10 GiB +128 MiB; minimum free space was 10,882,441,216 bytes. There was no disk stop or timeout. The guard required an additional 16 MiB at launch and bounded process-group shutdown. No Cargo/rustc process remained after completion.

[Exact proof receipt](proofs/java-menu-api-20261106/receipt.json) and [test log](proofs/java-menu-api-20261106/api-unit-result.log) retain the actual argv, hashes and local proof harness. Those harness files record this workspace-specific run; they are not installed mod modules or a replacement CI runner.

**Still pending on this combined tree:** the two consumer differential fixtures, and the separate complete official native plan/apply/type gate. The previous 24 Python checks and reproducible archive proof remain distinct. No Actions operation, game session, new material-display API or automatic crafting activation is included.


### Java consumer differential gate

On the same frozen combined source69c6, the real Java compatibility consumer and its frozen legacy oracle passed all **8 tests, 0 failed, 0 ignored**. The existing 1,311 differential cases retain ID-policy, UTF-16, spread, linked collision and resize coverage; TreeBin remains explicitly unsupported. This is comparison with the frozen Rust oracle, not a new JVM run.

The bounded run observed exactly one API library compilation and one Java test-harness compilation, with no other compilation. It finished in 2.60 seconds; minimum free space was 10,886,770,688 bytes. The 193-source/API/lock seal remained unchanged and no compiler process remained. [Java receipt](proofs/java-menu-api-20261106/java-receipt.json) and [log](proofs/java-menu-api-20261106/java-result.log) preserve this separate terminal result. Menu consumer and the full official native gate remain pending.


### Menu consumer differential gate and grouped result

The same frozen69c6 source also passed **11 menu consumer tests, 0 failed, 0 ignored**, including the real thin wrapper, its frozen legacy codec, 390 encoding comparisons and 7,600 malformed-frame comparisons. Exactly one menu test-harness compilation occurred; the actual API library from the immediately preceding Java stage was reused, with no other compilation. This was observed by the wrapper, not assumed from dependency metadata.

The separate locked/offline run finished in0.60 seconds. Minimum free space was10,885,468,160 bytes, with no guard stop or timeout. The193-source/API/lock seal remained unchanged and no compiler process remained. [Menu receipt](proofs/java-menu-api-20261106/menu-receipt.json) and [log](proofs/java-menu-api-20261106/menu-result.log) retain the exact result.

The grouped focused result is **125 API +8 Java +11 menu =144 Rust tests passed**, plus the earlier24 Python source/package checks and deterministic archive identity. These are different scopes and remain individually recorded. **The complete official native plan/apply/type gate is still pending for this combined tree.** No live JVM, game, LAN, automatic-crafting connection, material-display injection or old-PR change is claimed.
