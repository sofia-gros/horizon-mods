# Process-local UTF-8 for guarded Windows staging

Source-only follow-up to `f9fcdd39f13979af817ee4db970dcc43f58697d6`.
Only the launcher's child environment changes. No guard comparison, metadata,
formatter selection, stage source, Rust source, package or PowerShell entry is
changed. This is not another formatter/Windows/game execution result.

## Concrete failure

The unchanged damage guard uses `Path.read_text()` followed by
`sha256(text.encode())`. Its two official 41f source inputs contain 13 and 5
non-ASCII lines. Reading the actual complete files with CP1252 then encoding
UTF-8 changes their hashes to `1bc4a9c8...` and `5ec9b7a8...`, respectively.
Neither equals the real file hash. Thus fixing only the earlier source tests
would leave a real pre-apply Windows guard failure on a CP1252-default Python.

The adjacent test evidence contains exact official revision, full-file bytes
and both hashes, plus all 18 original non-ASCII lines with line numbers. Those
full-file observations were computed from the already verified official files.
The portable tests use the exact line bytes, not another official download or
a claim to have executed the guard on Windows.

## Narrow correction

`build_environment` removes inherited spellings of `PYTHONUTF8` and
`PYTHONIOENCODING` case-insensitively, then sets `PYTHONUTF8=1` and
`PYTHONIOENCODING=utf-8` in the returned environment. It does not mutate the
parent environment, registry, OS locale, installed Python or user configuration.
The stage child is launched with that environment; its guard child inherits it
and imports the same unchanged checkers. No `-E`, `-I` or opposing UTF-8 flag is
added. Other Python preparation children also receive the same environment.

`PYTHONUTF8` fixes the child interpreter's default text-file encoding;
`PYTHONIOENCODING` fixes its standard streams. Setting only the latter would not
fix `Path.read_text`. The current launcher already uses bytes or explicit UTF-8
for its own reads/captured outputs, so no late attempt to reconfigure its already
started interpreter is made. Its changed source is included in the existing
package-source/cache seal. This does not replace LF-preserving official checkout
preparation or permit changing source bytes to fit a hash.

## Validation boundary

Seven pure regressions cover environment override/isolation, exact inverse to
the prior launcher, fixed actual-source observations, CP1252 drift on all exact
Unicode lines, the unchanged damage digest, and stage-to-guard inheritance.
The prior 13 formatter and 20 latest-launch pure tests remain separate checks.
No child interpreter, native formatter, Cargo, planner, Windows API or game is
executed by these new tests. Actual Windows guard regeneration remains a later
gate, including exact Rust 1.99.1 formatter output equivalence.
