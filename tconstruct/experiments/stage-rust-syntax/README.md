# Generated Rust syntax gate

The normal stage now runs complete-plan guards, official apply, the exact known
one-space repair, then a non-writing Rust parser gate before returning to the
existing launcher/compiler. No official source, lock, feature or guard changes.
The launcher pins the changed stage SHA; its existing tools-tree source seal
includes the new helper. Existing CLI arguments remain unchanged.

The helper uses the same selected `PLUTO_GUARD_RUSTFMT` absolute executable as
the existing guards. Ordinary Linux callers retain the existing PATH fallback.
It sends the exact bounded target bytes through stdin with an empty explicit
config, edition 2024 and `--emit stdout`; stdout is discarded. No `--check`,
source filename, rewriting or additional module traversal is used. The fixed
Rustfmt implementation parses stdin and disables out-of-line module traversal
at formatting.rs:110–141. Local native testing also accepts an absent-module
sentinel through stdin, confirming the nonrecursive behavior.

The existing generated `worldgen/structure/start.rs` target is limited to 4 MiB.
The known formatter child has a 30-second timeout; diagnostics are polled and
limited to 1 MiB, with only 4096 characters retained. Timeout/budget failure is
not classified as Rust syntax rejection. The child is killed and reaped before
temporary-file cleanup on failure. The original target is never opened for
writing. Input and formatter bytes are rechecked after acceptance. A fresh,
read-back-checked `rust-syntax-receipt.json` records success or failure and
explicitly excludes type checking. The parser remains inside the existing
owned Windows Job when called by the bounded build driver.

Five existing files have strict whole-source inverses in inverse.json. Prior
source tests undo only this new exact layer before their original assertions.
The ordinary tiny Git/fake-CLI fixture uses an explicit syntax double after its
repair double; it never claims to exercise a real Rust parser.

## Verification

- 137 selected tests pass: previous 127 pure, 9 new pure tests and the existing
  tiny Git/fake-CLI fixture. All prior inverse/hash assertions remain.
- The actual proposed helper was separately executed with official Linux
  Rustfmt 1.99.1 on the full official source, an artificial bad copy, and its
  one-space repair. Outcomes were 0, 1 and 0; original bytes were preserved.
- The wider local probe had eight expected syntax outcomes plus a version call.
  See docs/local-rust-syntax-result-20261107.json for exact hashes and timings.
- An initial pure timeout-cleanup test accidentally matched its receipt file
  with a directory glob. The assertion was restricted to directories; runtime
  code was not changed for that test correction.

These are original-derived inputs, not the discarded generated Windows file.
No new complete plan, official apply, Windows build, type check or game was run.
The next bounded build must exercise this gate on its actual fresh staged file.
