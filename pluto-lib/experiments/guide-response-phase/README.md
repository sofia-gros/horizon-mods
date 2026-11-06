# Post-observer reply boundary: isolated constraint probes

These no-op manifests are excluded from the native package. They do not add a
registry, owned state, packet subscriber, page mutation or TConstruct activation.
The installed payload/current-book guards are unchanged. Run only as separate
read-only plans against the exact official pair in `sources.json`.

- `return-mut`: can the simple official `native_client_packet` wrapper expose a
  mutable receiver and its borrowed packet at RETURN? Acceptance would establish
  only a binding location after the original native dispatch, not permission to
  mutate a screen or compatibility with Pluto's existing exclusive RETURN.
- `after-call`: can official Mixins target `handle_packet`'s single
  `self.native_client_packet(&packet)` call without cancellation? The larger
  caller contains control-flow that may be unsupported for body rewriting.
  A rejection must be retained instead of bypassing source/guard constraints.

Any later service must keep native dispatch first, then the existing read-only
Pluto observations, then an explicitly scoped optional effect stage. It must not
cancel native handling, replay observation, copy packets into a later world, or
expose a raw mutable Game to the host-independent callback registry. No provider
must mean no extra view capture, state activation or mutation.

A mutable borrow is not UI authority. A future reply application capability
needs its own one-invocation, at-most-once contract plus a proven owned opening
and pending request. Current RequestTracker values are in-process correlation,
not native opening identity or a serializable transport envelope. A marker,
title/content equality, pointer or native level epoch alone cannot authorize a
late response. The existing lifetime experiment is still inactive and has no
live Client proof. TConstruct's LAN request/response path remains unchanged.

This group investigates a feature-preserving connection rather than disabling
remote refresh or weakening the existing mandatory guards. No successful plan
here may be reported as a completed connection or a full TConstruct check.

## Executed constraint proof

At source `647760f1`, `return-mut` passed in 8.612 seconds. `after-call`
was rejected in 6.607 seconds with `E_UNSUPPORTED_CONTROL_FLOW`; no plan
was produced for that attempt. Neither experiment was applied or typechecked.
No installed helper, guard, registry or TConstruct path changed. Exact source,
accepted binding, rejection and log hashes are in [validation.json](validation.json).

The [connection design](design.md) records the single-RETURN candidate and
remaining ownership, legacy-envelope size, document-budget, retained-overlay
and cover-position compatibility requirements. These are not completed APIs.

Seven source/metadata, in-memory package-exclusion and JSON-shape regressions
pass with `PYTHONDONTWRITEBYTECODE=1 python3 test_contract.py`. The size models
do not execute native Text or the Rust legacy codec. No package/stage is written
by these tests.
