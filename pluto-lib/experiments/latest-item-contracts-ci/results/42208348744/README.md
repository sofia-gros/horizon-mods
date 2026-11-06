# Latest Template and default-lifetime validation

The first run of the frozen candidate passed on official Horizon f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd.

- Source: [be7b6b98d4911270679fd35f85800186105b1715](https://github.com/sofia-gros/horizon-mods/commit/be7b6b98d4911270679fd35f85800186105b1715).
- [Run 42208348744](https://github.com/sofia-gros/horizon-mods/actions/runs/42208348744), job 127860516569, SUCCESS.
- Template: all six exact tests passed, zero failures/ignored/filtered.
- Default lifetime: all four exact tests passed in a separate binary, zero failures/ignored/filtered.
- Actual cold compilation of official horizon-core, horizon-client-core, horizon-items and horizon-world was witnessed in the Template build. The second harness reused that native graph; it has its own actual --test compiler witness.
- Both harnesses were compiled and sealed before direct binary execution. Separate inventories, adapted locks, source seals and fixture receipts are preserved below.
- All 23 phases exited zero. The final runner receipt reports passed=true and verified=true after official/source/OUT_DIR/binary verification.
- Pure controller/preparer checks also passed: 24 + 20 + 18.
- Minimum free space: 89,876,086,784 bytes; floor: 10,871,635,968 bytes.
- Retrieved console log: 48,319 UTF-8 bytes, SHA256 060f1ff0a4e4bf429e271cdabd009d82271c95429d93195b3b584af43db32828. Raw phase logs were temporary runner files; their hashes are provenance, not a promise that those logs remain downloadable.

## What this proves

The six finite Template tests exercise the actual source-extracted latest native Template implementation with actual native dependencies: absent/empty raw patches, explicit zero, removal precedence, cloning, the limited synthetic create-only factory case, and transient map postprocessing.

The four unchanged catalog tests exercise actual latest Items/defaults behavior: a catalog retains a translation key without materializing the standard item_name value, holding the shared defaults Arc delays same-root reload, Items does not itself keep that Arc alive, and display reads do not mutate the explicit stack patch.

The passing lifetime tests establish the current gap and ownership constraints. They do not implement the proposed Items-owned default-name payload.

## Not proved or activated

No Mixins plan/apply, ordinary combined package, native Game rendering, latest item_name removal codec32, automatic default-name population, or production activation occurred in this run. The two excluded fixture literal fixes remain an unapplied adaptation patch. This result does not replace the separate old-624c codec32 evidence.

The trigger branch remains frozen at be7b. This closure is a docs-only commit on the non-trigger source branch.

## Receipts

- [Final receipt](final-receipt.json)
- [Template six-test receipt](template6-receipt.json)
- [Default four-test receipt](default4-receipt.json)
