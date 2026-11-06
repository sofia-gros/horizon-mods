# Latest official revision-only diagnostic: timed out, compatibility unknown

The dedicated first run completed with GitHub conclusion **failure**. This is
an incomplete planner observation, not an official compatibility rejection and
not a successful migration.

- [Run 42201486827](https://github.com/sofia-gros/horizon-mods/actions/runs/42201486827), job 127837577332.
- Exact workflow head: `db54310784abe58ca51ac6991278e5b573e8f7ae`.
- Latest official Horizon: `f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`.
- Latest official Mixins: `5a64af938ec86b00d53d16a408aabc0038fb3e57`.
- Official plan: **180.231 seconds, timeout, exit -15**, empty diagnostic stream.
- `diagnostic_complete=false`, `source_verified=true`, `probe_completed=false`.
- `plan_accepted=null`, `compatibility=null`, `error=TimeoutError: official-plan`.

## What completed

28 pure Python checks, all eleven exact latest source/lock contracts, actual
official CLI compilation, and both original package reconstructions passed.
Original main 08afa992 produced SHA256
`8178b440266664d3db4dd9784af02894ebdb29910b0dd8f13504b2d341cf8f42`
(8,171,156 bytes, 499 members). Original common produced exact
`4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d`
(270,125 bytes, 147 members).

The original 644-entry composition was retained. The separate candidate changed
only the two declared manifest horizon_revision values, with exact before/after
hashes and complete inverse verification. Its other 642 entries were unchanged.
Raw archive Horizon/Mixins headers stayed untouched and were reported separately
from the actual latest workspace/tool. Candidate member-seal SHA256:
`692fa172e45e00dcddf62d201c6dbf206702f16e8b06a7109c8e6b6c13f0c125`.

The runner completed 21 phases, then timed out during its one official-plan
phase. Eight prerequisite gates passed. Its own process-group termination path
completed without a teardown error. Final source, Git, original/candidate,
wrapper, archive and CLI seals all matched. Remote minimum free space was
90,523,041,792 bytes and minimum observed available memory 13,533,388,800 bytes.

## What remains unproved

No official diagnostic code, mod/target/file, accepted plan or metadata subphase
was obtained. Console timestamps bound the plan phase from 10:19:39.7747326Z
to 10:22:40.0070035Z; CPU or internal subphase usage was not recorded. No
workflow artifacts were uploaded. The wrapper argv file is not included in the final receipt because
the plan call did not return; its call count is not asserted. The twelve old
guards, apply, host/consumer compilation and runtime behavior were not reached.
No timeout increase, automatic retry or guard relaxation was performed.

The prior unchanged-main E_REVISION observation remains separately preserved at
[08bf25d9](https://github.com/sofia-gros/horizon-mods/commit/08bf25d97c1295e83e6685ec3b1c9be86db41fd9).
Old 624c/e8 successes remain historical evidence only. This result changes no
main/PR, runtime body, old guard, installed Mod or original archive.

## Evidence preservation

The companion JSON contains the complete FINAL_RECEIPT and complete retrieved
job console, gzip+base64: **36,276 bytes**, SHA256
`bd8ef214c2cb1fac7887db5a994094550d2689803e0a770e1c3ceb48a76dde2a`. Decode base64 and gzip to recover exactly
those console bytes. Raw verbose phase files and generated binary/archive
payloads were not fetched; their receipt hashes are identity evidence, not a
reconstruction claim. Publication is documentation-only on a non-trigger branch.
