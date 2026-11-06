# UNAPPLIED player effect-instance proposal

This is an **unfinished upstream-facing prototype**, stored in horizon-mods.
It is not an installed PlutoLib service, approved upstream change, patch to an
official checkout, or live contact/cure implementation. Do not add it to the
ordinary package. No official repository may be modified or contacted by these
tools.

The [call-site inventory](../../docs/effect-extension-callsites.md) explains why
the current native value representation is retained for this bounded experiment,
instead of silently switching to the different shared-reference world kernel.

## Proposed code

`UNAPPLIED-player-instance.patch` is a generated diff against official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0`. It proposes a public field whose
validated owned type has private storage, preserving existing struct-update
callers. This is an upstream API proposal, not a Mixins state workaround.

- `extension.rs` provides opaque namespaced bytes owned by one instance. There
  is no address/ID side table. Caller-selected limits bound entries, bytes and
  hidden layers. Unknown namespaces are retained. Failed insertions leave the
  current map untouched. Namespaced byte encoding and limits are proposals, not
  existing native policy or an adopted public API.
- Native derived Clone continues to clone the whole value/hidden chain. Ordinary
  updates retain active data; the existing hidden-copy branches clone the right
  owner. Downgrade transfers the restored hidden payload. Native equality is
  explicitly kept on its original fields; payload comparison is separate.
- `Record` is an owned, flat active-then-hidden chain with an explicit registry
  resolver. It validates the complete input before native value construction and
  rejects unknown IDs or invalid amplifiers rather than dropping/clamping them.
  Bounds count logical IDs/keys/bytes, not encoded or heap size.

**Record conversion is not a save implementation.** The actual five-field
`PlayerEffects::saved/from_saved`, `Level.active_effects`, local/LAN persistence
and NBT codecs are deliberately unchanged. They still cannot retain the new
payload or hidden records. The proposal must not be enabled before a coherent
typed record migration and old-save/failure policy are implemented and reviewed.
Client packet replacement, same-invocation contact authority, item-cure ordering
and mob tuples also remain unresolved integrations.

## Reproducible bounded fixture

`prepare.py` reads only the approved clean official checkout, verifies its exact
commit and all inventoried source hashes, creates the unapplied diff in this
directory, and optionally creates a fixture under this MOD worktree's `.work`.
It has no apply/build mode and refuses output outside that `.work` directory.

The Rust fixture includes the exact native `EffectInstance` implementation up
to `EffectAction`, modified only by the displayed proposal. It removes the
unused potion-catalog import from that excerpt and adjusts the test-local module
path. It supplies no mock native EffectInstance, PlayerEffects or potion catalog.
This is proposed-source proof, not compilation of the full official crate.

Preparation (no native compilation):

```sh
python3 pluto-lib/experiments/effect-instance-proposal/prepare.py \
  --official /path/to/approved-clean-official-checkout \
  --output .work/effect-instance-prototype
```

Only after the coordinator grants a serial bounded slot, compile that prepared
fixture with the approved toolchain and run it. Preserve source hashes, command,
terminal output and the continuous disk-floor result. Check the prepared file
hashes again before and after execution. No native dependency graph, official
workspace modification, whole-host type claim or gameplay claim follows from
these tests.

Initial test source covers new/default construction, independent clone,
ordinary update, both hidden-copy branches, hidden restoration, recursive hidden
update, proposed-record roundtrip, unknown payloads, bounds, unresolved IDs and
atomic invalid insertion. Execution status belongs in a separate evidence
record; the presence of test source is not a passing result.

## Observed bounded result

At local source `257377be7d2f1798763623b5d578783be6f98ef6` (remote checkpoint
`debfa2cf8df25745dede6b06b53b188dfe170790`), the std-only proposed-source fixture
compiled and all **10 tests passed**, with none skipped. Four Python source
guards also passed. The continuous disk guard did not stop execution; its
minimum was 10,885,697,536 bytes. Official inputs and all prepared file hashes
were reverified before and after. See [validation.json](validation.json) for
commands, exact hashes and test names. This result does not close any of the
excluded native integration gates above.
