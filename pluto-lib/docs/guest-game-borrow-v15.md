# Applied-v15 Game borrow probe runner: prepared, not executed

**UNAPPLIED / UNFINISHED test-only work.** This runner has not copied a stage,
invoked Cargo, or observed a compiler rejection. It is not installed in PlutoLib
and does not supply a contact continuation capability.

## Exact target and separation from pristine evidence

The copy source is the coordinator's passed applied-v15 stage:

- Runtime source: `f5a83539525c26fd2142e26e43789a3dd0107033`.
- Package SHA-256: `7d9c4c5d7bd91ec6584b9206658f9d5bdb5d04dd3f95f1cfe90ceca6daae5386`.
- Published integration checkpoint: `89744814`.
- Stage: `/tmp/pluto-v15-host-proof-20261106/staged`, 5,047 regular files.
- Source-seal SHA-256: `e16a3781aae1dea6d1a627f4bd28641469e16e822dae9704d0507a75a8d02428`.
- Applied Game SHA-256: `e02eef2d2d72b5e01b6ef9efc46de5652f33270966b2fbbb91398035d4cb3fdb`.

The 19 pristine official sources remain separately pinned evidence for the
proposed lexical shape and semantics. They are **not** asserted identical to
applied v15. In particular pristine Game has a different hash. The new
`UNAPPLIED-v15-game-borrow-{positive,negative}.patch` artifacts append exactly
the existing private probe modules to applied Game. They do not reuse a pristine
patch as evidence that applied source was unchanged.

## Prepared execution protocol

`run_game_borrow_v15.py` defaults to read-only verification. `--write-artifacts`
only updates the two MOD-owned unapplied diffs. `--execute` requires a separate
coordinator grant after other native jobs end.

The execution path is limited to:

1. Verify the complete v15 seal, all original mtimes, passed gate metadata,
   package identity, pristine evidence and exact probe bytes.
2. Copy regular files with `copy2` into a new direct `/tmp` evidence directory.
   No hardlinks, official writes, old stage writes, cache deletion, forged cache
   fingerprints or changed manifests. Preserve unchanged-file mtimes only by
   copying their exact bytes and attributes.
3. Claim the existing logical Cargo alias with exclusive symlink creation. Do
   not replace another owner's alias. Compare actual Cargo metadata package IDs,
   manifests, targets and features to v15, under the unchanged lock.
4. Append the positive module only. Run locked/offline `cargo check` on the real
   `horizon-client` **library only**, no default features. Require terminal zero
   and an actual `rustc` invocation for that library in verbose output.
5. Verify all sources, then replace the positive addition with the mutually
   exclusive negative addition against the same original Game. Require exit
   101, exactly one compiler error, E0499, actual gameplay library target and
   primary span on the added `game.add_player_effect(effect);` line. Unrelated
   errors, absent diagnostics, stale outputs and disk stops do not count.
6. Verify every overlay file including Cargo.lock, all original v15 files and
   mtimes, pristine evidence and experiment inputs. Require native generated
   OUT_DIR bytes used by the actual compiler to match pre-run/v15 evidence.
   Remove only this runner's exact temporary alias in `finally`. Preserve both
   logs, source manifests, structured rejection and the final negative overlay.

The continuous guard retains 10 GiB + 128 MiB on persistent disk and 512 MiB in
`/tmp`. Guarded file copying reserves the full 119,175,755-byte source payload
before starting; the tmp copy plus logs is roughly 120 MB. Cargo uses the existing
shared target with one job, no debug information and no incremental output.
Incremental cache growth is not guaranteed to fit the current narrow persistent
headroom: a guard stop is a blocked attempt, never positive or negative proof.
The runner terminates only its own process group.

## Prepared validation and excluded claims

Ten Python checks passed. These include exact applied target/diffs, separate
pristine evidence, source/lock protection, exclusive alias cleanup, positive-first
control flow, and synthetic diagnostic-parser rejection cases. Synthetic JSON
only validates the parser; no E0499 has yet been observed.

A successful future positive check would establish **compile acceptance against
this actual applied-v15 Game composition**. It would not prove that a guest can
be resolved at runtime, that outer owner state is restored, or that a native
Inside frame can resume after teleport, replacement, death or reentrant action.
The negative result would only demonstrate why a still-live MotionWorld borrow
cannot coexist with the whole-Game effect call. It would not implement the
required frame split. Mob roster and rider loans remain a separate unresolved
owner path. No runtime test, contact hook or effect activation is included.
