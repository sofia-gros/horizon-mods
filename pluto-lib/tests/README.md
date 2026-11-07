# Shared API checks

Run `python3 -m unittest discover -s pluto-lib/tests -v` with the pinned Rust
toolchain on PATH. Source/package checks need Python 3.11 or later. The inventory
API test runs the complete real crate in a temporary directory and uses
`inventory-api.Cargo.lock`, whose registry package identities were taken from
the official Horizon 624c4f62 integration lock (the first complete source cutoff
is recorded in the foundation validation reports).

Tests default to offline, locked Cargo execution. On a cold machine, explicitly
set `PLUTO_TEST_FETCH_LOCKED=1` to fetch only the committed lock first; CI does
this before the same offline locked test. This test lock is not packaged into
the native bundle. Official Mixins still resolves the bundled crate against the
selected host's lockfile, and actual package plan/apply/type-check is a separate
gate. When changing dependencies or the selected host, regenerate this lock from
that host, verify every registry identity/checksum, and rerun the tests. Do not
silently regenerate it against an arbitrary newer registry version.
