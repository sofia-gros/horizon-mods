# PR945 storage/mining adaptation boundary

Read-only source comparison, not a new plan or build. Exact PR945 head is `d4d798fbbdc9dbbe0f81adb0e4074c798737e5ad`, reported base PR944 is `bcec17b5394aba2a58f20e9f77222424f9b2ad6f`. Prior full646 was built from main `08afa992`, so it is a different composition.

The old two mining-hook raw block and chest-hook raw block each occur exactly once and unchanged in PR945. Old host/client mining helpers, Items RETURN, rules/math and chest helper are byte-identical. The new mining facts/HEAD and owned chest helper can therefore be considered as the same narrow local adaptations, but require a new full-base seal/inverse; the existing transformers intentionally reject PR945 as a different input.

Two dependencies must not be reverted:
- Shared mining apply now calls the actual common public `pluto_fluid_facts::player` once and reads current/previous water facts. The common4f13 adapter computes exactly the original `is_eye_in_fluid(WATER)` and `fluids.was_eye_in_water` fields. This is source evidence, not a latest Game type proof.
- The melter storage module has moved `write_many` staging to existing `pluto_inventory::storage::stage_updates`. The source diff is limited to that method's staging block; the chest's metadata read is unchanged.

Both public module owners are present in the actual common4f13 archive, rather than merely a source directory excluded from packaging. Preserve these PR945 changes, navigation and all other unrelated production bytes. Do not transplant the previous main08afa shared-policy bytes over them. Old36 differential cases cover unchanged rules/math and bounded arithmetic, not this new Game composition.

Mining b78 has formal-entry evidence and pure36 policy evidence; chest68bdd has16 pure source tests and41f small probe source40a0, but no actual chest IR or Game type result yet. Full latest plan/guards/apply/build and a launch remain outstanding. The integration branch is owned by the coordinator; this report performs no integration or PR action.
