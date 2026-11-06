# Latest mining complete-candidate diagnostic result

diagnostic complete; official full plan rejected; compatibility false.

[Run 42215886835](https://github.com/sofia-gros/horizon-mods/actions/runs/42215886835), job `127885664173`, source `74fc9a53e4bde161f326bbd16710b625f86716e6`. GitHub job conclusion: `success`. This does not certify apply, native typing, gameplay or full compatibility.

- Receipt: diagnostic_complete=True, source_verified=True, probe_completed=True, plan_accepted=False, compatibility=False.
- Error: `None`.
- Recorded phases: 24; gates: 12.
- Minimum free disk 90508034048 bytes; minimum observed available memory 13521154048 bytes.

Original archives reproduced and exact 644→646 candidate inverse verified: `True`. Candidate member seal `02882fbd01e9dc5f06a3cd61d53ae9497dcabfd8302ebf98bba99892d9ceb823`; changed paths: `pluto_native_creative/horizon-mixin.toml`, `pluto_native_creative/mining-context.rs`, `tconstruct_native/horizon-mixin.toml`, `tconstruct_native/source-mining-context-native.rs`.

Actual monitored plan phase: 580.106 seconds, exit `1`, error `RuntimeError: official-plan exited 1`.
Actual first official diagnostic:

```json
{
  "code": "E_UNSUPPORTED_CONTROL_FLOW",
  "file": "crates/horizon-client/src/game/block_items.rs",
  "injection_id": "tinkers-chest-pick-color",
  "message": "closures, async, try/?, unsafe and const blocks are unsupported injection contexts",
  "mod_id": "tconstruct_native",
  "target": "horizon_client::game::block_items::Game<'_>::picked_stack"
}
```

Terrain collector was not reached; no rendering identity witness is claimed.

Old12 guards were not reached; no guard/type/apply success is claimed.

## Preservation and limits

Full fetched console: 54,994 bytes, SHA256 `b794d7a06a22ae769ced2942d0a66368f449a53384dc071d96ed9265d71490ff`. The adjacent JSON embeds its complete gzip/base64 bytes and the FINAL_RECEIPT parsed directly in Python to preserve nanosecond integers. Any reconstructed tool/CPU records match their recorded byte count and SHA256.

Original main `08afa992` and successful common `4f13a6e2` remain immutable. The separate candidate uses exact b78 private facts/helper bytes on latest Horizon f4dc / Mixins5a64; full archive and final source seals are in the receipt. There was one plan with a 600-second ceiling and no retry or guard relaxation.

The small formal probe and scalar36 proof remain separate. Native adapter visibility/typing, actor-frame paths, local/guest/prediction behavior and authoritative mutation are not proved here. Whole plan bytes beyond retained selected IR were ephemeral. Full phase raw logs were not fetched, and their hashes do not recreate them.
