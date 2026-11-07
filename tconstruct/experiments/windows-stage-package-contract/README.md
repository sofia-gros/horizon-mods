# Windows stage full-package contract caller

Source-only candidate based on integration c1b54ce8e1a66b087c43cdefd030ed998810e35c.
It must be adopted with the Recipe owner's reviewed guard group. Its shared writer
is check_recipe_placement_plan.py, SHA256
7a153e6a4863a3721ead6aa8b96b189b35a2df7bc7464f44950a1184e9cd0496.
That dependency was copied locally for the pure test and is not republished here.
This checkpoint alone does not add or replace the Recipe guard.

The existing Windows launcher passes both same-checkout hnmod paths and their
expected SHA256 values from its previously verified package receipt. Direct stage
callers must now provide --tconstruct-archive, --pluto-archive,
--tconstruct-archive-sha256 and --pluto-archive-sha256. Paths must be absolute.
The launcher generates these arguments automatically; no extra user prompt or
manual schema construction is introduced.

stage_package_contract.py reuses the existing strict package inspector and full
member map. It checks both expected archive hashes and all expanded files before
loading the shared writer. The writer alone defines the full-tconstruct schema.
The resulting bounded UTF-8 JSON is created exclusively, fsynced, and read back.
Archives, the contract, original expanded members and copied combined members are
rechecked before plan, before the mandatory guard, and after the guard before apply.
The existing check_native_plan.py receives that JSON as its third argument. A
failure remains fatal. No standalone fallback, guard skip, planner substitution,
Rust setting, source pin, recipe authority, or runtime hook is added here.

Archive limits retain the existing inspector's 64 MiB/512 entries, 64 MiB total
unpacked bytes and separate support limits; the shared Recipe writer also imposes
its stricter 16 MiB archive/1 MiB manifest budget. Contract JSON has a 1 MiB limit.
Stat/rechecks assume quiescent inputs and reject ordinary drift. They are not a
claim of protection against adversarial filesystem races.

Stage and launcher changes have exact whole-source inverses to c1b54. Historical
source tests first verify and undo this later delta. The link/UTF-8 historical
checks also undo the already-adopted, separately reviewed stat delta. Original
inverse metadata, historical CP1252 evidence, and their expected hashes remain
unchanged. The link mock supplies the two receipt hashes now required by the real
caller. These are tests, not new production fallback behavior.

Validation is pure Python: new 13 cases (including the actual shared writer with
synthetic packages), existing launch 20, mocked link 9, stat 12, UTF-8 7 and package
11. They do not invoke a real packager, Git, Cargo, rustfmt, planner, apply, Windows,
linker or game. Separate historical cache suite: 10 pass and 2 errors on both this
candidate and unchanged c1b54, because its old PowerShell inverse witness predates
the current entry. Those unchanged paths are not repaired in this limited delta.
The initial sparse recovery also lacked its two inverse JSON dependencies; after
restoring both exact c1b54 files, the remaining same baseline errors were recorded.

No actual complete-plan/guard/Windows/type/link compatibility is claimed. Existing
formatter retry eb19 still stopped at version with 0 output bytes and wrapper exit
4294967295; this connection neither diagnoses nor fixes that independent failure.
