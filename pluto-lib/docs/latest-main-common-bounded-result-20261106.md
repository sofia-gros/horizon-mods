# Latest official bounded diagnostic: first actual target rejection

[Run 42204940940](https://github.com/sofia-gros/horizon-mods/actions/runs/42204940940),
job 127849104018, exact head `ada5a94348885cec83cce94461f9d51d6dff05c7`.
GitHub conclusion **success** means diagnostic collection and verification
completed. **Compatibility is false**; no compatibility repair is claimed.

## Actual result

The one official plan ran for **575.217 seconds**, within the separately
approved 600-second bound, and returned **exit 1**:

- Code: `E_LOCAL_NOT_FOUND`.
- Message: `no local declaration speed:0`.
- Mod: `tconstruct_native`.
- Injection: `source-host-mining-context`.
- Target: `horizon_client::game::Game<'_>::tick_mining`.
- File: the separate candidate's `tconstruct_native/horizon-mixin.toml`.

`diagnostic_complete=true`, `source_verified=true`, `probe_completed=true`,
`plan_accepted=false`, `compatibility=false`, `error=null`.
The twelve existing native guards, apply and host/consumer compilation were
**not reached**. No accepted plan.json, node/fingerprint subset or runtime proof
was produced. This identifies the first real rejection, not every later blocker.

## Unchanged inputs and preservation checks

Main remained `08afa992d5091e03360f3473ea93fd17aa793987`; its actual package
reproduced `8178b440266664d3db4dd9784af02894ebdb29910b0dd8f13504b2d341cf8f42`.
The successful historical common package reproduced
`4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d`.
Official Horizon was `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`; official
Mixins was `5a64af938ec86b00d53d16a408aabc0038fb3e57`.

40 pure regressions, eleven latest official contract files, actual CLI tool
compilation, both complete original archives, the original 644-member tree,
the separate two-manifest revision-only candidate and full byte inverse all
passed. Raw old archive headers were retained. All final source/Git/original/
candidate/wrapper/archive/CLI seals matched. There were nine diagnostic gates
and 22 phases: 21 succeeded, then the planner's expected diagnostic exit 1.

## Bounded observations

One exact allowed metadata **call-start witness** was recorded under the
metadata-only wrapper. It is not independently a completion marker. No compiler
or unexpected tool invocation was allowed after CLI tool construction.

The same-group observer recorded **115 direct-CLI CPU samples**, bounded to one
known PID with parent/group/session/birth checks. At elapsed 570.372 seconds,
user ticks were 56848 and system ticks 164, with 100 ticks per second. The last
sample omits the final interval and all descendants. It does not identify a
specific internal state/module phase or prove a timeout cause. The observer
reported CLI exit 1 at 575.176 seconds, and both CLI and observer exited1. Their
output streams preserved the native 382-byte diagnostic unchanged.

The final receipt retains all samples and call records. Remote minimum available
memory was 13,539,319,808 bytes and free disk 90,522,918,912 bytes. No memory, disk,
permission or guard relaxation occurred. The own process-group teardown and
final verification completed.

## Exact retained evidence and limits

The companion JSON includes the full receipt and complete retrieved console:
**49,336 bytes**, SHA256 `5c7aba2d544259fffcf2c62b2e61ae87c2aa53909eb1d93f10eff4d2cc45bc98`.
It also includes deterministic reconstructions of the 10,376-byte CPU JSONL,
335-byte wrapper JSONL and 382-byte raw official diagnostic, each independently
matched to the runner's size/SHA256. Decode each gzip+base64 field to recover
those bytes. Other verbose phase logs and generated package binaries were not
fetched; their hashes alone are not a reconstruction claim.

The earlier 180-second timeout remains unmodified at
[ac311972](https://github.com/sofia-gros/horizon-mods/commit/ac311972dfb0bce806e35bb2dc209a059a52871f).
No further retry, main/PR change, guard weakening or application followed this
result. This closure is documentation-only on a non-trigger branch.
