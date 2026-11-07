# Windows runtime-preparation test failure, 2026-11-07

[Run42331766258](https://github.com/sofia-gros/horizon-mods/actions/runs/42331766258), head d91738ac, stopped in the new pure suite: 15passed/1error. The earlier33controller and12stat tests passed. The failure was KeyError '/' in a test that assumed a POSIX filesystem key on Windows. No rustc archive download, formatter preparation, version or7regenerations was reached.

The same child/wrapper both exited1. All3owned Jobs were empty and final source/artifact verification passed; the overall result remained false. The fix changes only the test's expected key to str(Path('/tmp').anchor), keeping the80-byte mocked minimum and every other assertion.

Observed minimum free bytes were C:33,488,343,040 and D:157,730,607,104. Minimum available physical RAM was14,179,172,352B over43samples. These measurements precede component acquisition and compilation; they do not establish heavy-build growth or readiness. Exact observations and log identity are in the adjacent JSON. No raw console is republished. Runtime, URLs, budgets, Job and guard inputs are unchanged, and no automatic retry was made.
