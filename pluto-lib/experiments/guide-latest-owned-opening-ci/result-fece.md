# Latest Guide attempt: inventory-format mismatch

Exact `fece0572e74fb73ea114bdf88cadeb54f2c3514d`, run
[42210426433](https://github.com/sofia-gros/horizon-mods/actions/runs/42210426433),
ended **failure**. The fresh complete 21-source inventory passed again. The
explicitly typed std fixture compiled successfully and its list invocation
exited 0. The runner then rejected the output because it required a summary
line even though the requested Rust `--list --format terse` output has none.

This is supported by exact evidence, not a guessed format: the 17 pinned names,
sorted and each followed by `: test\n`, reproduce the actual phase's 1,589
UTF-8 bytes and SHA-256
`6c8956115e348eda2885f926e55c035df43121dd67652cd2ee4100f09c6956c0`.
The reconstructed full listing is retained in adjacent JSON. The raw phase file
was not downloaded; its console-recorded hash and size match this reconstruction.

The binary is 7,005,160 bytes, SHA-256
`b91ecb95cd406e01d85e72279e10b26832a356258d115fc9d390f12e0a77ad8c`.
No selected test executed. No experimental host copy, metadata comparison or
actual Game/BookScreen type check was reached. The final receipt remains false;
its source-seal verification is not a successful complete gate.

A parser-only follow-up should compare the complete terse output against all
17 exact names, rejecting extra text, duplicates, missing rows and other formats.
It must not weaken the subsequent exact 10-pass/7-filtered execution gate.
Existing runtime fragments, helper/output hashes, ordinary manifests and LAN
remain unchanged. Any retry needs a separate source review and serial release.
