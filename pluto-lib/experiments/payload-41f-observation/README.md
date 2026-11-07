# Actual 41f payload observation

This is a metadata-only follow-up to the pending payload guard. It does not alter the checker algorithm, either payload helper, any manifest, or the Guide/broad/haste consumers.

The fresh complete plan is from run 42328100345, controller 7082868bcbf37ed3eabd32f40ddf0cf08a4ee30c, and integration e9073648b3c4cf4d49f27bc2e1ed60997f430586 at official Horizon 41f822b659789780f58d32c63ddfb6887ea1b8bf / Mixins 5a64af938ec86b00d53d16a408aabc0038fb3e57. Its 6,431,102 bytes have SHA256 dc4fbc3c54144f2f95728986b4e9bfb3700235655b18a3af3c8d1a6113dcb86b. The original twelve diagnostics reported eight passes and four refusals. Payload refused the deliberately missing observation. That historical result is not rewritten as a pass.

The actual plan contains exactly one reviewed Pluto native_client_packet RETURN, node [40,349], with the original bindings and helper bytes. The private updater has its canonical owner/ID, game.rs placement, name and source bytes. All six handle_packet HEAD hooks are noncancellable. In particular, Guide/broad/haste no longer cancel the native path. All four official input hashes and both packaged helper hashes match.

A separate read-only tool using the real official Index resolved the known observer and handle_packet controls and poll_remote. The poll_remote fingerprint is e1b7b4869ed3ffcebf6acf7e70cd9acfd66acfbe2363e46abf901a6489a80aac at remote.rs node [44,41]. The receipt reports actual core and harness compilation, successful observation, and verified sources. The original 930-byte stdout was not separately downloaded. Canonical JSON reconstructed from the saved result object matches the reported 930-byte SHA256 7f3180ff5ed8c00d65820684fd179cfa684e07144ffd37dd7806b971bb9cf205 exactly; evidence.json distinguishes that reconstruction from an independently retrieved stdout file.

Only the previously missing poll fingerprint, candidate observation record and scope description change in the metadata. The exact prior metadata is recovered by clearing those two observation values and restoring its prior coverage text. The checker continues to reject extra/aliased observer hooks, the old cancellable callers, missing/duplicate/foreign updater modules, source drift, and absent observations.

The local evidence replay requires GUIDE_PLAN_DIR (the verified artifact directory), GUIDE_INDEX_RECEIPT (the saved auxiliary receipt), and GUIDE_OFFICIAL_ROOT (the exact four official source files). Run python3 -B test_observation.py -v from this directory. These are read-only Python guard and source checks. The test file does not fetch, build, apply, or execute native code.

This does not establish successful apply, actual Game typing, the six internal Game tests, actor/opening lifetime, real UI/language reload, or LAN delivery. The RETURN runs after native_client_packet's body but before the remaining handle_packet match. Restoring the native/read-only observers is an intentional difference from the old cancelled path. Same-sequence reapplication and the existing legacy packet/body limits remain unchanged.

## Local replay result

The unchanged checker passes the complete saved actual plan and all four exact official source files after this metadata update. Four evidence/regression tests and eleven existing synthetic guard/source tests pass. The four tests include six changed-full-plan refusals and both missing-observation refusals. The separate packaged unit test was not run. Evidence closure: [c50b6acb](https://github.com/sofia-gros/horizon-mods/blob/c50b6acbe7f7eebe1032149dbcbf8cf6033b61f4/pluto-lib/docs/playtest-next-plan-result-20261107.json).
