# Commonization checkpoint, 2026-11-06

This development branch preserves committed work; it is not a release or a completed TConstruct recovery.

The latest complete ordinary-library proof is cutoff `31cf76672d3a7b982aebab86a91a78440afd163e` (v13), archive SHA-256 `20aa2a95fd066b9714bd6068ad7f654391a32b52d4a2e42aa3b791f5720bbce5`, against official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`. That exact package passed plan, all 11 required composition guards, apply and client library/binary checks. Its 193 Python checks invoke the actual fluid API (43 tests) and inventory API (115 tests).

Later committed candidates include the bounded terrain-cell adapter, explicit crafting product/remainder staging, native Font layout queries, and a further chest pickup helper delegation. Their focused evidence is recorded in the corresponding documents. They have NOT yet passed the next combined ordinary-library apply/type gate. The terrain-cell adapter adds a twelfth required complete-plan guard. CPU rendering equivalence, GPU behavior and automatic product-pair consumption remain unproved/unactivated.

The existing TConstruct `materials-book-response` cancellable `handle_packet` hook and `materials-book-turn` hook conflict with the mandatory client-payload/current-book guards. The full TConstruct + Pluto staging/CI route therefore remains blocked. Do not bypass these guards, treat this branch as main-ready, or infer full consumer, live gameplay, save/reload or LAN correctness from the ordinary-library proof.

The isolated guide lifetime experiment remains excluded from ordinary native packaging. EffectInstance attachment has an observed official E_STATE rejection; the reusable curative values do not attach to native instances. All work uses the official upstream source pins, never the prohibited user Horizon fork.
