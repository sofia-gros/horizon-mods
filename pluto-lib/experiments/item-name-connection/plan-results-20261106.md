# Item-name connection: inert plan results

The exact `0fba3bd003ab56a6a53c28647d85dd30c63deb5e` no-op source passed all
three read-only plans against official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`
and Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02` on 2026-11-06.
This establishes target/selector/binding planning, not Rust type acceptance,
semantic behavior, complete-package composition or production activation.

## Accepted targets

1. Receive: `MODIFY_EXPRESSION_VALUE` of the original `components_tag(...)`
   result, once in `item_sync::from_network` and once in
   `item_sync::template_tag`. The second route covers the nested-template
   conversion seam that does not call `from_network`.
2. Send: cancellable `HEAD` at the actual recursive
   `lan::guest::network_stack`. The current helper always continues; no stack
   is cloned, normalized or sent by the probe.
3. Defaults: `WRAP_OPERATION` of `Items::load`'s single
   `Self::load_with` call. The current helper invokes its continuation exactly
   once. It does not read, retain or alter default components.

The [structured result](plan-results-20261106.json) records all four exact
native target fingerprints, selectors, plan hashes and input hashes. Actual
plan sizes were 609,505 / 608,429 / 608,896 bytes. No source helper, native
source or CLI bytes changed during execution.

## Initial failure and bounded retry

The first three attempts all ended before target resolution with `E_METADATA`:
`failed to start cargo metadata: No such file or directory (os error 2)`.
The runner had not supplied the existing toolchain PATH. Those diagnostics
are preserved and are not classified as rejection of the proposed seams.

The single authorized environment-only retry used the existing toolchain and
a CARGO wrapper that accepted only the exact native planner command:
`metadata --format-version 1 --no-deps --manifest-path <official>/Cargo.toml
--offline --locked`. The compiled planner dependency's actual command builder
was read first to verify it honors CARGO. All three recorded invocations
matched; build, test, check and online fetch were not allowed.

Each plan finished in under 9 seconds within its 90-second bound. Outputs were
only in a new dedicated temporary directory, 1,828,535 bytes before the small
receipt, below the 3 MiB budget. The persistent filesystem minimum was
10,878,734,336 bytes, above its 10 GiB + 128 MiB floor; the temporary filesystem
kept more than 2 GiB free. Own process groups were reaped, the prohibited target
directory was absent, and complete official/source/CLI seals matched afterward.

## Still required

No production manifest was changed and no proposed removal/default behavior
was enabled. The subsequent implementation still needs an explicit
added-versus-removed conflict contract, native raw-Wire and canonical-send
proofs, nested item preservation, default snapshot ownership, actual type
checks and composition guards. A successful inert plan does not settle the
proposed use of ItemInfo storage or the public display contract.
