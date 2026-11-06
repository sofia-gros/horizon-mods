# Windows playtest Pluto cache inputs

Prepared against the exact integration source `e128d61478971e247b1e3e2e09516cf99652f9fc`.
This is a source/Python-tested change, not a successful PowerShell execution,
Windows build, latest complete-plan admission or playable release.

That integration already has `plan -> check_native_plan.py -> apply` in stage.py;
it is unchanged here. Old main08afa had direct apply, but that historical defect
must not be described as still present in this integration. The official launcher
does not run these repository-specific guards; its native-build documentation is
not permission to bypass this development bundle's guarded staging route.

The remaining cache defect was independent: the selected external Pluto bundle
was chosen only after computing the build key. Only creative.py from Pluto was
in that key, so different selected bundle/guard bytes could reuse an existing
ready build. Selection now happens before the key. A read-only helper hashes:

- All regular files in the selected bundle, with relative names and byte lengths.
- This checkout's reviewed `pluto-lib/mixins/pluto_native_creative` source tree.
- This checkout's `pluto-lib/tools` guard scripts/metadata and `pluto-lib/pluto_lib`.

The digest is added to the existing key. It is recomputed before writing `.ready`
and before launching either a new or cached executable. A changed digest aborts;
no automatic deletion, rebuild retry or guard bypass occurs. The picker currently
runs on every invocation so the intended bundle is explicit. Identical file bytes
remain identical despite mtime changes. Interpreter `.pyc`/`__pycache__` only are
excluded; links/reparse points are rejected before that exclusion. Empty directories
are not content inputs. Limits are 10,000 files, 64MiB per file, 256MiB summed over
the labelled roots (overlapping roots count twice). Exceeding a limit rejects.

Regular-file checks include every root ancestor and descendant. Windows reparse
attributes and POSIX symlinks/special files reject. File stat/open/post-read checks
catch ordinary drift. This is not a concurrent malicious-filesystem guarantee;
use quiescent source and bundle folders. Inputs must still pass the separate
manifest, complete-plan, guard and compiler checks before a new build is marked
ready. This fingerprint is not an authorization token or a proof of compatibility.

Twelve pure Python regressions pass: source/guard mutation, add/delete/rename,
mtime stability, interpreter cache, missing inputs, symlink parents/files,
modeled Windows reparse bits, special files, budgets, ordinary file drift, exact
PowerShell edit reversal and key/ready/launch order. They do not execute PowerShell,
exercise real NTFS reparse handling, compile Rust or validate Windows gameplay.

Unchanged limitations include the existing resource-folder-path key (not a full
prepared-resource seal), user-managed compiler environment, no-default-features
playtest build, and absent latest complete-plan/Windows startup evidence. These
are separate acceptance gates; the new Pluto digest does not solve them.
