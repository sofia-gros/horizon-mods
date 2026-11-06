# Windows formatter first attempt

Exact controller 38e1b11df5191197e3f6129959d8c84fd4b666aa ran once in [run 42253259752](https://github.com/sofia-gros/horizon-mods/actions/runs/42253259752), job 128013622320, and failed at 2026-11-06 16:32:33 UTC.

The source preflight and fixed historical artifact download succeeded. All 20 named pure tests passed. Preparation then rejected the freshly written rustfmt.exe at the path-stat/open-handle-stat comparison: `Formatter input changed before read`. Both owned Windows Jobs were empty at termination and final source verification passed.

No formatter version or regeneration ran. This is not evidence of a rustfmt output mismatch, missing DLL, complete current composition, apply, Windows linking or gameplay. Archive hash/layout validation precedes the reached source frame, but there was no completed preparation receipt or recorded executable-byte admission. The original false outcome is retained without retry.

The decoded job log is retained locally; its UTF-8 bytes and digest are in the JSON summary. It is not republished here. The current signature compares st_mode across different Windows APIs. CPython 3.12.10 source adds filename-derived execute bits for path stat of .exe/.bat/.cmd/.com, while handle fstat does not. This is a concrete source-level explanation to test; the failed run did not record numeric stat fields, so no other identity differences are assumed away.

Sources: [path mode adjustment](https://github.com/python/cpython/blob/v3.12.10/Modules/posixmodule.c), [handle fstat](https://github.com/python/cpython/blob/v3.12.10/Python/fileutils.c). A follow-up must preserve exact same-API checks, file identity/type/size/timestamps and no-link admission; it must not weaken the guard oracle or silently substitute another formatter.
